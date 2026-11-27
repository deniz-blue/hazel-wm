local focus = require("focus")
local floating = require("floating")
local requests = require("requests")
local lock = require("lock")
local M = {}

local style = {
	height = 24,
	radius = 6,
	edge = 2,
	close_width = 24,
	padding = 10,
	focused = "#88c0d0",
	unfocused = "#3b4252",
	text_focused = "#2e3440",
	text_unfocused = "#d8dee9",
	font = { size = 13 },
	close_font = { size = 18 },
}

local bars = {}
local exclusions = {}

--- Windows for which `excluded(window)` is true get no title bar.
function M.exclude(excluded)
	table.insert(exclusions, excluded)
end

local function wants_bar(window)
	if not (floating.is_floating(window) and window:mapped()) then return false end
	local states = window:states()
	if states:has("fullscreen") or states:has("maximized") then return false end
	for _, excluded in ipairs(exclusions) do
		if excluded(window) then return false end
	end
	return true
end

local function colours(focused)
	if focused then return style.focused, style.text_focused end
	return style.unfocused, style.text_unfocused
end

local function title_of(window)
	return window:title() or window:app_id() or ""
end

local function title_limit(width)
	return math.max(width - style.padding - style.close_width, 0)
end

local function centred_y(font, text)
	local metrics = wm.text:measure(text, font)
	return -style.height + (style.height - math.floor(metrics.height)) // 2
end

local function close_x(width)
	local metrics = wm.text:measure("×", style.close_font)
	return width - style.close_width + (style.close_width - math.floor(metrics.width)) // 2
end

local function create(window, width, focused, title)
	local fill, text_colour = colours(focused)
	local x = -style.edge
	return {
		width = width,
		focused = focused,
		title = title,
		background = window:add_shape({
			kind = "rect", x = x, y = -style.height, width = width + 2 * style.edge, height = style.height,
			radius = style.radius, fill = fill, z = 1,
		}),
		filler = window:add_shape({
			kind = "rect", x = x, y = -style.radius, width = width + 2 * style.edge, height = style.radius,
			fill = fill, z = 1,
		}),
		label = window:add_shape({
			kind = "text", x = style.padding, y = centred_y(style.font, title), text = title, font = style.font,
			color = text_colour, max_width = title_limit(width), z = 2,
		}),
		close = window:add_shape({
			kind = "text", x = close_x(width), y = centred_y(style.close_font, "×"), text = "×", font = style.close_font,
			color = text_colour, z = 2,
		}),
	}
end

local function remove(id)
	local bar = bars[id]
	if not bar then return end
	for _, name in ipairs({ "background", "filler", "label", "close" }) do bar[name]:remove() end
	bars[id] = nil
end

local function update(bar, window, width, focused, title)
	if bar.width ~= width then
		bar.background:update({ width = width + 2 * style.edge })
		bar.filler:update({ width = width + 2 * style.edge })
		bar.label:update({ max_width = title_limit(width) })
		bar.close:update({ x = close_x(width) })
		bar.width = width
	end
	if bar.focused ~= focused then
		local fill, text_colour = colours(focused)
		bar.background:update({ fill = fill })
		bar.filler:update({ fill = fill })
		bar.label:update({ color = text_colour })
		bar.close:update({ color = text_colour })
		bar.focused = focused
	end
	if bar.title ~= title then
		bar.label:update({ text = title })
		bar.title = title
	end
end

local function sync(window)
	local id = window:id()
	if not wants_bar(window) then
		remove(id)
		return
	end
	local width = window:size().width
	local focused = #window:focused_seats() > 0
	local title = title_of(window)
	if bars[id] then
		update(bars[id], window, width, focused, title)
	else
		bars[id] = create(window, width, focused, title)
	end
end

--- Brings every title bar in line with its window; only what changed is sent to the compositor.
function M.refresh()
	for _, window in ipairs(wm.windows:all()) do sync(window) end
end

local function stack_index(window)
	for i, other in ipairs(wm.windows:all()) do
		if other == window then return i end
	end
	return 0
end

--- The window whose title bar is at `position`, and whether the point is on its close button.
function M.at(position)
	local windows = wm.windows:all()
	for i = #windows, 1, -1 do
		local window = windows[i]
		if bars[window:id()] then
			local at, size = window:position(), window:size()
			local inside = position.x >= at.x and position.x < at.x + size.width
				and position.y >= at.y - style.height and position.y < at.y
			if inside then
				local covering = wm.windows:under(position)
				if covering and covering ~= window and stack_index(covering) > i then return nil end
				return window, position.x >= at.x + size.width - style.close_width
			end
		end
	end
end

focus.claim_presses(function(pointer) return M.at(pointer:position()) ~= nil end)

wm.input:on("pointer_button", lock.unless_locked(function(event)
	if event.state ~= "Pressed" or event.button ~= Button.Left then return end
	local window, on_close = M.at(event.pointer:position())
	if not window then return end
	event:prevent_default()
	window:raise()
	event.pointer:seat():focus(window)
	if on_close then
		window:close()
	else
		floating.begin_move(window, event.pointer)
	end
end))

wm.windows:on("mapped", M.refresh)
wm.windows:on("focus", M.refresh)
wm.windows:on("title_changed", M.refresh)
wm.windows:on("resized", function(window) sync(window) end)
wm.windows:on("closed", function(window) remove(window:id()) end)
floating.on_change(M.refresh)
requests.on_change(M.refresh)

M.refresh()

return M
