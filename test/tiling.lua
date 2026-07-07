--[[
tiling.lua — tiling layouts grouped by workspaces.

Usage:
  require("tiling")
  tiling.add_workspace("name", "layout")
  tiling.active("name")
  tiling.set_layout("name", "layout")

Layouts: master_stack, even_grid, horizontal, vertical, monocle
--]]

-- ═══════════════════════════════════════════════════════════════════════════
-- Layouts
-- ═══════════════════════════════════════════════════════════════════════════

local layouts = {}

function layouts.master_stack(windows, area)
	if #windows == 0 then return end
	if #windows == 1 then
		windows[1]:set_position(Point(area.x, area.y))
		windows[1]:set_size({ width = area.width, height = area.height })
		return
	end
	local ratio = 0.6
	local mw = math.floor(area.width * ratio)
	local sw = area.width - mw
	windows[1]:set_position(Point(area.x, area.y))
	windows[1]:set_size({ width = mw, height = area.height })
	local sh = math.floor(area.height / (#windows - 1))
	local sy = area.y
	for i = 2, #windows do
		local w = windows[i]
		w:set_position(Point(area.x + mw, sy))
		w:set_size({ width = sw, height = sh })
		sy = sy + sh
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
		w:set_position(Point(area.x + c * cw, area.y + r * ch))
		w:set_size({ width = cw, height = ch })
	end
end

function layouts.horizontal(windows, area)
	if #windows == 0 then return end
	local w = math.floor(area.width / #windows)
	for i, win in ipairs(windows) do
		win:set_position(Point(area.x + (i - 1) * w, area.y))
		win:set_size({ width = w, height = area.height })
	end
end

function layouts.vertical(windows, area)
	if #windows == 0 then return end
	local h = math.floor(area.height / #windows)
	for i, win in ipairs(windows) do
		win:set_position(Point(area.x, area.y + (i - 1) * h))
		win:set_size({ width = area.width, height = h })
	end
end

function layouts.monocle(windows, area)
	if #windows == 0 then return end
	windows[1]:set_position(Point(area.x, area.y))
	windows[1]:set_size({ width = area.width, height = area.height })
	for i = 2, #windows do
		windows[i]:set_position(Point(-9999, -9999))
		windows[i]:set_size({ width = 1, height = 1 })
	end
end

-- ═══════════════════════════════════════════════════════════════════════════
-- State & helpers
-- ═══════════════════════════════════════════════════════════════════════════

local workspaces = {}
local order = {}
local default = nil

local function work_area()
	local outputs = wm.outputs:all()
	if #outputs == 0 then return nil end
	local r
	for _, out in ipairs(outputs) do
		local pos, sz = out:position(), out:size()
		if pos and sz then
			local r2 = Rect(pos.x, pos.y, sz.width, sz.height)
			r = r and r:merge(r2) or r2
		end
	end
	return r
end

local function apply_one(name)
	local ws = workspaces[name]
	if not ws then return end
	local fn = layouts[ws.layout]
	if not fn then return end

	local all = wm.windows:all()
	local live = {}
	for _, w in ipairs(ws.windows) do
		for _, cur in ipairs(all) do
			if w == cur then
				table.insert(live, w)
				break
			end
		end
	end
	ws.windows = live
	if #live == 0 then return end
	local area = work_area()
	if not area then return end
	fn(live, area)
end

local function apply_all()
	for _, name in ipairs(order) do
		apply_one(name)
	end
end

-- ═══════════════════════════════════════════════════════════════════════════
-- Events
-- ═══════════════════════════════════════════════════════════════════════════

wm.windows:on("new_window", function(window)
	local ws = workspaces[default]
	if not ws then return end
	table.insert(ws.windows, window)
	apply_one(default)
end)

wm:on("tick", function()
	local all = wm.windows:all()
	local changed = false
	for _, name in ipairs(order) do
		local ws = workspaces[name]
		local i = 1
		while i <= #ws.windows do
			local alive = false
			for _, cur in ipairs(all) do
				if ws.windows[i] == cur then
					alive = true
					ws.windows[i] = cur
					break
				end
			end
			if not alive then
				table.remove(ws.windows, i)
				changed = true
			else
				i = i + 1
			end
		end
	end
	if changed then apply_all() end
end)

-- ═══════════════════════════════════════════════════════════════════════════
-- API
-- ═══════════════════════════════════════════════════════════════════════════

local M = {}

function M.add_workspace(name, layout)
	if workspaces[name] then return end
	workspaces[name] = { name = name, layout = layout or "master_stack", windows = {} }
	table.insert(order, name)
	default = default or name
end

function M.set_layout(name, layout)
	local ws = workspaces[name]
	if ws and layouts[layout] then
		ws.layout = layout
		apply_one(name)
	end
end

function M.active(name)
	if workspaces[name] then default = name end
end

--- Force re-layout all workspaces right now.
function M.retile()
	apply_all()
end

function M.list()
	local r = {}
	for _, name in ipairs(order) do
		local ws = workspaces[name]
		if ws then
			table.insert(r, { name = ws.name, layout = ws.layout, count = #ws.windows })
		end
	end
	return r
end

function M.debug()
	local area = work_area()
	print("tiling: workspaces=" .. #order .. " windows=" .. wm.windows:count() .. " area=" .. tostring(area))
	for _, name in ipairs(order) do
		local ws = workspaces[name]
		print("  " .. name .. " (" .. ws.layout .. "): " .. #ws.windows .. " windows")
	end
end

-- ═══════════════════════════════════════════════════════════════════════════
-- Bootstrap
-- ═══════════════════════════════════════════════════════════════════════════

M.add_workspace("1", "master_stack")
M.add_workspace("2", "even_grid")
M.add_workspace("3", "monocle")

return M
