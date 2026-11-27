--[[
tiling.lua — tiling layouts grouped by workspaces, part of the example compositor.

Usage:
  local tiling = require("tiling")
  tiling.add_workspace("name", "layout")
  tiling.active("name")             -- on the active output
  tiling.active("name", output)     -- on a given output
  tiling.set_layout("name", "layout")
  tiling.move_focused_to("name")
  tiling.focus_next(1)

Layouts: master_stack, even_grid, horizontal, vertical, monocle
Every output shows one workspace and lays it out in its own usable area; a workspace shown
nowhere is hidden. Showing a workspace that another output shows swaps the two.
--]]

local seats = require("seats")
local outputs = require("outputs")

-- ═══════════════════════════════════════════════════════════════════════════
-- Layouts
-- ═══════════════════════════════════════════════════════════════════════════

local layouts = {}

local gaps = { inner = 0, outer = 0 }

local function place(window, x, y, width, height)
	local inset = gaps.inner // 2
	window:set_position(Point(x + inset, y + inset))
	window:set_size({ width = math.max(width - 2 * inset, 0), height = math.max(height - 2 * inset, 0) })
end

local function inset_area(area)
	local margin = gaps.outer - gaps.inner // 2
	return Rect(area.x + margin, area.y + margin, math.max(area.width - 2 * margin, 0), math.max(area.height - 2 * margin, 0))
end

function layouts.master_stack(windows, area)
	if #windows == 0 then return end
	if #windows == 1 then
		place(windows[1], area.x, area.y, area.width, area.height)
		return
	end
	local mw = math.floor(area.width * 0.6)
	place(windows[1], area.x, area.y, mw, area.height)
	local sh = math.floor(area.height / (#windows - 1))
	for i = 2, #windows do
		place(windows[i], area.x + mw, area.y + (i - 2) * sh, area.width - mw, sh)
	end
end

function layouts.even_grid(windows, area)
	if #windows == 0 then return end
	local cols = math.ceil(math.sqrt(#windows))
	local rows = math.ceil(#windows / cols)
	local cw = math.floor(area.width / cols)
	local ch = math.floor(area.height / rows)
	for i, w in ipairs(windows) do
		local r, c = math.floor((i - 1) / cols), (i - 1) % cols
		place(w, area.x + c * cw, area.y + r * ch, cw, ch)
	end
end

function layouts.horizontal(windows, area)
	if #windows == 0 then return end
	local w = math.floor(area.width / #windows)
	for i, win in ipairs(windows) do
		place(win, area.x + (i - 1) * w, area.y, w, area.height)
	end
end

function layouts.vertical(windows, area)
	if #windows == 0 then return end
	local h = math.floor(area.height / #windows)
	for i, win in ipairs(windows) do
		place(win, area.x, area.y + (i - 1) * h, area.width, h)
	end
end

function layouts.monocle(windows, area, focused)
	if #windows == 0 then return end
	local top = focused or windows[1]
	for _, win in ipairs(windows) do
		place(win, area.x, area.y, area.width, area.height)
		win:set_visible(win == top)
	end
end

-- ═══════════════════════════════════════════════════════════════════════════
-- State & helpers
-- ═══════════════════════════════════════════════════════════════════════════

local workspaces = {}
local order = {}
local shown = {}
local orphans = {}
local tracking_focus = true
local moved_listeners = {}
local change_listeners = {}

local exclusions = {}
local hiders = {}

local function is_hidden(window)
	for _, hides in ipairs(hiders) do
		if hides(window) then return true end
	end
	return false
end

local function is_tiled(window)
	local states = window:states()
	if states:has("fullscreen") or states:has("maximized") then return false end
	if is_hidden(window) then return false end
	for _, excluded in ipairs(exclusions) do
		if excluded(window) then return false end
	end
	return true
end

local function find_workspace_of(window)
	for _, name in ipairs(order) do
		for i, w in ipairs(workspaces[name].windows) do
			if w == window then return workspaces[name], i end
		end
	end
end

local function workspace_shown_on(output_name)
	return shown[output_name] and workspaces[shown[output_name]]
end

local function output_showing(workspace_name)
	for output_name, name in pairs(shown) do
		if name == workspace_name then return output_name end
	end
end

local function untiled_windows(ws)
	local found = {}
	for _, window in ipairs(ws.windows) do
		if not is_tiled(window) then table.insert(found, window) end
	end
	return found
end

--- Windows that no layout places keep their offset inside the output when they change outputs.
local function carry_untiled(windows, from, to)
	if not (from and to) or from.name == to.name then return end
	local before, after = from:area(), to:area()
	for _, window in ipairs(windows) do
		local position = window:position()
		window:set_position(Point(position.x - before.x + after.x, position.y - before.y + after.y))
		for _, listener in ipairs(moved_listeners) do listener(window) end
	end
end

local function apply_workspace(ws, output)
	local fn = layouts[ws.layout]
	local area = outputs.usable_area(output)
	if not fn or #ws.windows == 0 then return end
	local tiled = {}
	for _, w in ipairs(ws.windows) do
		if is_tiled(w) then table.insert(tiled, w) end
	end
	fn(tiled, inset_area(area), ws.last_focused)
end

local function apply_visibility()
	tracking_focus = false
	for _, name in ipairs(order) do
		local visible = output_showing(name) ~= nil
		local ws = workspaces[name]
		for _, w in ipairs(ws.windows) do
			if is_hidden(w) then
				w:set_visible(false)
			elseif not (visible and ws.layout == "monocle" and is_tiled(w)) then
				w:set_visible(visible)
			end
		end
	end
	tracking_focus = true
end

local function retile()
	apply_visibility()
	for _, output in ipairs(wm.outputs:all()) do
		local ws = workspace_shown_on(output.name)
		if ws then apply_workspace(ws, output) end
	end
	for _, listener in ipairs(change_listeners) do listener() end
end

local function focus_workspace_window(ws)
	local target = ws.last_focused or ws.windows[1]
	if target then seats.active():focus(target) end
end

local function active_workspace()
	local output = outputs.active()
	return output and workspace_shown_on(output.name)
end

local function first_unshown_workspace()
	for _, name in ipairs(order) do
		if not output_showing(name) then return workspaces[name] end
	end
end

local function new_workspace_name()
	local n = #order + 1
	while workspaces[tostring(n)] do n = n + 1 end
	return tostring(n)
end

local function give_workspace(output, may_create)
	local ws
	while #orphans > 0 and not ws do
		local candidate = workspaces[table.remove(orphans, 1)]
		if candidate and not output_showing(candidate.name) then ws = candidate end
	end
	ws = ws or first_unshown_workspace()
	if not ws and may_create then
		local name = new_workspace_name()
		workspaces[name] = { name = name, layout = "master_stack", windows = {} }
		table.insert(order, name)
		ws = workspaces[name]
	end
	if ws then shown[output.name] = ws.name end
end

--- Outputs without a workspace get one nobody shows; with `may_create`, a new one when all are shown.
local function give_every_output_a_workspace(may_create)
	local present = {}
	for _, output in ipairs(wm.outputs:all()) do
		present[output.name] = true
		if not workspace_shown_on(output.name) then give_workspace(output, may_create) end
	end
	for name in pairs(shown) do
		if not present[name] then shown[name] = nil end
	end
end

-- ═══════════════════════════════════════════════════════════════════════════
-- Events
-- ═══════════════════════════════════════════════════════════════════════════

wm.windows:on("new_window", function(window)
	local ws = active_workspace() or workspaces[order[1]]
	if not ws then return end
	table.insert(ws.windows, window)
	ws.last_focused = window
	local output = output_showing(ws.name) and wm.outputs:name(output_showing(ws.name))
	if output then
		local area = outputs.usable_area(output)
		window:set_position(Point(area.x, area.y))
	end
	retile()
end)

wm.windows:on("closed", function(window)
	local ws, index = find_workspace_of(window)
	if not ws then return end
	table.remove(ws.windows, index)
	if ws.last_focused == window then ws.last_focused = ws.windows[1] end
	retile()
end)

wm.windows:on("focus", function(window)
	if not window or not tracking_focus then return end
	local ws = find_workspace_of(window)
	if not ws then return end
	ws.last_focused = window
	if ws.layout == "monocle" then retile() end
end)

wm.outputs:on("added", function(output)
	if not workspace_shown_on(output.name) then give_workspace(output, true) end
	retile()
end)

--- The workspace a removed output showed gives its windows to the workspace another output shows.
wm.outputs:on("removed", function(output)
	local leaving = workspace_shown_on(output.name)
	shown[output.name] = nil
	if not leaving then return end
	local survivor
	for _, other in ipairs(wm.outputs:all()) do
		if other.name ~= output.name and (not survivor or other.name == (outputs.active() or {}).name) then survivor = other end
	end
	local receiving = survivor and workspace_shown_on(survivor.name)
	if not receiving then
		table.insert(orphans, leaving.name)
		retile()
		return
	end
	carry_untiled(untiled_windows(leaving), output, survivor)
	for _, window in ipairs(leaving.windows) do table.insert(receiving.windows, window) end
	receiving.last_focused = leaving.last_focused or receiving.last_focused
	leaving.windows, leaving.last_focused = {}, nil
	retile()
	focus_workspace_window(receiving)
end)

-- ═══════════════════════════════════════════════════════════════════════════
-- API
-- ═══════════════════════════════════════════════════════════════════════════

local M = {}

--- Windows for which `excluded(window)` is true are not laid out.
function M.exclude(excluded)
	table.insert(exclusions, excluded)
end

--- Windows for which `hidden(window)` is true stay invisible on every workspace.
function M.keep_hidden(hidden)
	table.insert(hiders, hidden)
end

--- `listener(window)` runs after a window that no layout places was carried to another output.
function M.on_moved_between_outputs(listener)
	table.insert(moved_listeners, listener)
end

--- `listener()` runs after every layout.
function M.on_change(listener)
	table.insert(change_listeners, listener)
end

--- Space left between windows and around the edge of the usable area.
function M.set_gaps(inner, outer)
	gaps = { inner = inner, outer = outer or inner }
	retile()
end

function M.add_workspace(name, layout)
	if workspaces[name] then return end
	workspaces[name] = { name = name, layout = layout or "master_stack", windows = {} }
	table.insert(order, name)
	give_every_output_a_workspace(false)
end

function M.set_layout(name, layout)
	local ws = workspaces[name]
	if not (ws and layouts[layout]) then return end
	ws.layout = layout
	if output_showing(name) then retile() end
end

--- Shows workspace `name` on `output` (the active output when it is left out).
function M.active(name, output)
	local ws = workspaces[name]
	output = output or outputs.active()
	if not (ws and output) or shown[output.name] == name then return end
	local displaced = workspace_shown_on(output.name)
	local elsewhere = output_showing(name)
	if elsewhere then
		local other = wm.outputs:name(elsewhere)
		shown[elsewhere] = displaced and displaced.name or nil
		if displaced then carry_untiled(untiled_windows(displaced), output, other) end
		carry_untiled(untiled_windows(ws), other, output)
	end
	shown[output.name] = name
	retile()
	focus_workspace_window(ws)
end

--- The name of the workspace `output` shows (the active output when it is left out).
function M.current(output)
	output = output or outputs.active()
	return output and shown[output.name]
end

function M.move_window(window, name)
	local target = workspaces[name]
	if not target then return end
	local ws, index = find_workspace_of(window)
	if not ws or ws == target then return end
	table.remove(ws.windows, index)
	if ws.last_focused == window then ws.last_focused = ws.windows[1] end
	table.insert(target.windows, window)
	target.last_focused = window
	local from, to = output_showing(ws.name), output_showing(target.name)
	if from and to then carry_untiled({ window }, wm.outputs:name(from), wm.outputs:name(to)) end
	retile()
	if from then focus_workspace_window(ws) end
end

function M.move_focused_to(name)
	local window = seats.focused_window()
	if window then M.move_window(window, name) end
end

function M.focus_next(direction)
	local ws = active_workspace()
	if not ws or #ws.windows == 0 then return end
	local focused = seats.focused_window()
	local _, index = find_workspace_of(focused)
	local count = #ws.windows
	local next_index = index and ((index - 1 + direction) % count) + 1 or 1
	local target = ws.windows[next_index]
	ws.last_focused = target
	if ws.layout == "monocle" then retile() end
	seats.active():focus(target)
end

function M.swap_master()
	local ws = active_workspace()
	local window = seats.focused_window()
	if not ws or not window then return end
	local _, index = find_workspace_of(window)
	if not index or index == 1 then return end
	ws.windows[index], ws.windows[1] = ws.windows[1], ws.windows[index]
	retile()
end

--- Shows the next workspace (the one before for -1) that no other output is showing.
function M.cycle_workspace(direction)
	local output = outputs.active()
	local current = output and shown[output.name]
	if not current then return end
	local index = 1
	for i, name in ipairs(order) do
		if name == current then index = i end
	end
	for step = 1, #order - 1 do
		local name = order[((index - 1 + direction * step) % #order) + 1]
		if not output_showing(name) then
			M.active(name, output)
			return
		end
	end
end

--- Force re-layout right now.
function M.retile()
	retile()
end

--- Every workspace: `name`, `layout`, `count` of windows, `output` (the name of the output showing it, or nil)
--- and `active` (shown on the active output).
function M.list()
	local active = outputs.active()
	local r = {}
	for _, name in ipairs(order) do
		local ws = workspaces[name]
		local output = output_showing(name)
		table.insert(r, {
			name = ws.name,
			layout = ws.layout,
			count = #ws.windows,
			output = output,
			active = output ~= nil and active ~= nil and output == active.name,
		})
	end
	return r
end

--- The name of the workspace a window is on.
function M.workspace_of(window)
	local ws = find_workspace_of(window)
	return ws and ws.name
end

function M.debug()
	print("tiling: workspaces=" .. #order .. " windows=" .. wm.windows:count())
	for _, info in ipairs(M.list()) do
		print("  " .. info.name .. " (" .. info.layout .. "): " .. info.count .. " windows, shown on " .. tostring(info.output))
	end
end

-- ═══════════════════════════════════════════════════════════════════════════
-- Surviving a reload
-- ═══════════════════════════════════════════════════════════════════════════

local function ids_of(windows)
	local ids = {}
	for _, window in ipairs(windows) do table.insert(ids, window:id()) end
	return ids
end

wm:on("unload", function()
	local saved = { order = order, shown = shown, orphans = orphans, gaps = gaps, workspaces = {} }
	for name, ws in pairs(workspaces) do
		saved.workspaces[name] = {
			layout = ws.layout,
			windows = ids_of(ws.windows),
			last_focused = ws.last_focused and ws.last_focused:id() or nil,
		}
	end
	wm.store.tiling = saved
end)

--- Puts the windows that exist now back on the workspaces they were on before the reload,
--- and the workspaces back on the outputs that showed them. Call it once the workspaces have been added.
function M.restore()
	local saved = wm.store.tiling
	if saved then
		for _, name in ipairs(saved.order) do
			M.add_workspace(name, saved.workspaces[name].layout)
			local ws, info = workspaces[name], saved.workspaces[name]
			ws.layout = info.layout
			for _, id in ipairs(info.windows) do
				local window = wm.windows:by_id(id)
				if window then table.insert(ws.windows, window) end
			end
			ws.last_focused = info.last_focused and wm.windows:by_id(info.last_focused) or nil
		end
		shown = {}
		for output_name, name in pairs(saved.shown or {}) do
			if workspaces[name] and wm.outputs:name(output_name) and not output_showing(name) then shown[output_name] = name end
		end
		orphans = saved.orphans or {}
		gaps = saved.gaps or gaps
		give_every_output_a_workspace(true)
	end
	local fallback = active_workspace() or workspaces[order[1]]
	for _, window in ipairs(wm.windows:all()) do
		if not find_workspace_of(window) and fallback then
			table.insert(fallback.windows, window)
		end
	end
	retile()
end

-- ═══════════════════════════════════════════════════════════════════════════
-- Bootstrap
-- ═══════════════════════════════════════════════════════════════════════════

M.add_workspace("1", "master_stack")
M.add_workspace("2", "even_grid")
M.add_workspace("3", "monocle")

return M
