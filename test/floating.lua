local lock = require("lock")
local M = {}

local MINIMUM_WIDTH = 100
local MINIMUM_HEIGHT = 60
local DIALOG_OFFSET = 40

wm.store.floating = wm.store.floating or {}
local floating = wm.store.floating
local listeners = {}
local drag = nil

local function notify()
	for _, listener in ipairs(listeners) do listener() end
end

function M.on_change(listener)
	table.insert(listeners, listener)
end

function M.is_floating(window)
	return floating[window:id()] == true
end

function M.set_floating(window, enabled)
	if M.is_floating(window) == enabled then return end
	floating[window:id()] = enabled or nil
	notify()
end

function M.toggle(window)
	M.set_floating(window, not M.is_floating(window))
end

local function movable(window)
	local states = window:states()
	return not (states:has("fullscreen") or states:has("maximized"))
end

local function edges_from_name(name)
	return {
		left = name:find("left") ~= nil,
		right = name:find("right") ~= nil,
		top = name:find("top") ~= nil,
		bottom = name:find("bottom") ~= nil,
	}
end

local function edges_nearest(window, point)
	local position, size = window:position(), window:size()
	return {
		left = point.x < position.x + size.width / 2,
		right = point.x >= position.x + size.width / 2,
		top = point.y < position.y + size.height / 2,
		bottom = point.y >= position.y + size.height / 2,
	}
end

local function begin(window, pointer, edges)
	if drag or not movable(window) then return false end
	M.set_floating(window, true)
	drag = {
		window = window,
		pointer = pointer,
		edges = edges,
		origin = pointer:position(),
		position = window:position(),
		size = window:size(),
	}
	if edges then window:states():set("resizing", true) end
	pointer:grab()
	return true
end

function M.begin_move(window, pointer)
	return begin(window, pointer, nil)
end

function M.begin_resize(window, pointer, edges)
	return begin(window, pointer, edges or edges_nearest(window, pointer:position()))
end

function M.finish()
	if not drag then return end
	if drag.edges then drag.window:states():set("resizing", false) end
	drag.pointer:ungrab()
	drag = nil
end

local function resized_axis(start, length, delta, near_edge, far_edge, minimum)
	if near_edge then
		local size = math.max(minimum, length - delta)
		return start + length - size, size
	elseif far_edge then
		return start, math.max(minimum, length + delta)
	end
	return start, length
end

local function update(position)
	local dx, dy = position.x - drag.origin.x, position.y - drag.origin.y
	local window, start, size, edges = drag.window, drag.position, drag.size, drag.edges
	if not edges then
		window:set_position(Point(start.x + dx, start.y + dy))
		return
	end
	local x, width = resized_axis(start.x, size.width, dx, edges.left, edges.right, MINIMUM_WIDTH)
	local y, height = resized_axis(start.y, size.height, dy, edges.top, edges.bottom, MINIMUM_HEIGHT)
	window:set_position(Point(x, y))
	window:set_size({ width = width, height = height })
end

local function window_under(pointer)
	local target = wm.input:target_under(pointer:position())
	return target and target:kind() == "window" and target or nil
end

wm.input:on("pointer_button", lock.unless_locked(function(event)
	if drag and event.state == "Released" and #event.pointer:buttons() == 0 then
		M.finish()
		event:prevent_default()
	end
end))

function M.bind_mouse(modifier, move_button, resize_button)
	wm.input:on("pointer_button", lock.unless_locked(function(event)
		if drag then return end
		if event.state ~= "Pressed" or not event.modifiers[modifier] then return end
		local window = window_under(event.pointer)
		if not window then return end
		local started
		if event.button == move_button then
			started = M.begin_move(window, event.pointer)
		elseif event.button == resize_button then
			started = M.begin_resize(window, event.pointer)
		end
		if started then event:prevent_default() end
	end))
end

wm.input:on("pointer_move", lock.unless_locked(function(event)
	if drag then update(event.position) end
end))

wm:on("session_locked", M.finish)

wm.windows:on("move_request", function(request)
	M.begin_move(request.window, request.pointer)
end)

wm.windows:on("resize_request", function(request)
	M.begin_resize(request.window, request.pointer, edges_from_name(request.edges))
end)

wm.windows:on("parent_changed", function(window)
	local parent = window:parent()
	if not parent then return end
	local position = parent:position()
	window:clear_size()
	window:set_position(Point(position.x + DIALOG_OFFSET, position.y + DIALOG_OFFSET))
	M.set_floating(window, true)
end)

wm.windows:on("closed", function(window)
	if drag and drag.window == window then M.finish() end
	floating[window:id()] = nil
end)

return M
