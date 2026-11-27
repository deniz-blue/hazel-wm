local seats = require("seats")
local outputs = require("outputs")
local M = {}

wm.store.requests = wm.store.requests or { saved = {}, minimized = {}, minimized_count = 0 }
local saved = wm.store.requests.saved
local minimized = wm.store.requests.minimized
local listeners = {}

local function notify()
	for _, listener in ipairs(listeners) do listener() end
end

local function fill(window, area)
	local id = window:id()
	if not saved[id] then
		local position, size = window:position(), window:size()
		saved[id] = { x = position.x, y = position.y, width = size.width, height = size.height }
	end
	window:set_position(Point(area.x, area.y))
	window:set_size({ width = area.width, height = area.height })
end

local function restore(window)
	local before = saved[window:id()]
	saved[window:id()] = nil
	if not before then return end
	window:set_position(Point(before.x, before.y))
	if before.width > 0 and before.height > 0 then
		window:set_size({ width = before.width, height = before.height })
	else
		window:clear_size()
	end
end

function M.on_change(listener)
	table.insert(listeners, listener)
end

function M.set_maximized(window, maximized)
	window:states():set("maximized", maximized)
	local output = window:output()
	if maximized and output then
		fill(window, outputs.usable_area(output))
	elseif not window:states():has("fullscreen") then
		restore(window)
	end
	notify()
end

function M.set_fullscreen(window, fullscreen, requested_output)
	window:states():set("fullscreen", fullscreen)
	local output = requested_output or window:output()
	if fullscreen and output then
		fill(window, output:area())
		window:raise()
	elseif window:states():has("maximized") then
		M.set_maximized(window, true)
		return
	else
		restore(window)
	end
	notify()
end

--- Makes a fullscreen or maximised window fill the output it is on now.
function M.refit(window)
	local output = window:output()
	if not output then return end
	if window:states():has("fullscreen") then
		fill(window, output:area())
	elseif window:states():has("maximized") then
		fill(window, outputs.usable_area(output))
	end
end

function M.is_minimized(window)
	return minimized[window:id()] ~= nil
end

function M.set_minimized(window, enabled)
	local id = window:id()
	if (minimized[id] ~= nil) == enabled then return end
	if enabled then
		wm.store.requests.minimized_count = wm.store.requests.minimized_count + 1
		minimized[id] = wm.store.requests.minimized_count
		local seats_on_window = window:focused_seats()
		window:set_visible(false)
		for _, seat in ipairs(seats_on_window) do seat:focus(wm.windows:topmost()) end
	else
		minimized[id] = nil
	end
	notify()
	if not enabled then seats.active():focus(window) end
end

function M.restore_last_minimized()
	local latest_id, latest_order = nil, 0
	for id, order in pairs(minimized) do
		if order > latest_order then latest_id, latest_order = id, order end
	end
	local window = latest_id and wm.windows:by_id(latest_id)
	if window then M.set_minimized(window, false) end
end

function M.toggle_maximized(window)
	M.set_maximized(window, not window:states():has("maximized"))
end

function M.toggle_fullscreen(window)
	M.set_fullscreen(window, not window:states():has("fullscreen"))
end

wm.windows:on("maximize_request", function(window) M.set_maximized(window, true) end)
wm.windows:on("unmaximize_request", function(window) M.set_maximized(window, false) end)
wm.windows:on("fullscreen_request", function(window, output) M.set_fullscreen(window, true, output) end)
wm.windows:on("minimize_request", function(window) M.set_minimized(window, true) end)
wm.windows:on("unminimize_request", function(window) M.set_minimized(window, false) end)
wm.windows:on("unfullscreen_request", function(window) M.set_fullscreen(window, false) end)

wm.windows:on("decoration_request", function(window)
	window:set_decoration("server")
end)

wm.windows:on("closed", function(window)
	saved[window:id()] = nil
	minimized[window:id()] = nil
end)

return M
