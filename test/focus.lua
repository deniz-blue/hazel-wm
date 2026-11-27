local lock = require("lock")
local seats = require("seats")
local M = {}

local last_focused = {}

local function focus_topmost(seat)
	seat:focus(wm.windows:topmost())
end

local function wants_keyboard(layer)
	local depth = layer:layer()
	return (depth == "top" or depth == "overlay") and layer:keyboard_interactivity() == "exclusive"
end

local claims = {}

--- Presses for which `claims(pointer)` is true do not move focus; whoever claims them decides.
function M.claim_presses(claims_press)
	table.insert(claims, claims_press)
end

local function claimed(pointer)
	for _, claims_press in ipairs(claims) do
		if claims_press(pointer) then return true end
	end
	return false
end

function M.focus_under(pointer)
	if claimed(pointer) then return end
	local seat = pointer:seat()
	local target = wm.input:target_under(pointer:position())
	if not target then
		seat:focus(nil)
	elseif target:kind() == "window" then
		target:raise()
		seat:focus(target)
	elseif target:keyboard_interactivity() ~= "none" then
		seat:focus(target)
	end
end

local following = false

function M.follow_mouse(enabled)
	wm.store.follow_mouse = enabled
end

local function follow(pointer, position)
	local seat = pointer:seat()
	local target = wm.input:target_under(position)
	if not target or target == seat:focused() then return end
	if target:kind() == "layer" and target:keyboard_interactivity() == "none" then return end
	following = true
	seat:focus(target)
	following = false
end

--- A window is Activated while any seat has it focused.
local function update_activation()
	for _, window in ipairs(wm.windows:all()) do
		window:set_activated(#window:focused_seats() > 0)
	end
end

wm.windows:on("new_window", function(window)
	seats.active():focus(window)
end)

wm.windows:on("focus", function(window, seat)
	last_focused[seat.name] = window or last_focused[seat.name]
	update_activation()
	if window and not following then window:raise() end
end)

wm:on("session_unlocked", function()
	for _, seat in ipairs(wm.input.seats:all()) do focus_topmost(seat) end
end)

wm.windows:on("closed", function(window)
	for _, seat in ipairs(wm.input.seats:all()) do
		if last_focused[seat.name] == window and not seat:focused() then focus_topmost(seat) end
	end
end)

local ACTIVATION_TOKEN_MAX_AGE_MS = 10000

wm.windows:on("activation_request", function(window, token_age_ms)
	if token_age_ms <= ACTIVATION_TOKEN_MAX_AGE_MS then seats.active():focus(window) end
end)

wm.layers:on("mapped", function(layer)
	if wants_keyboard(layer) then seats.active():focus(layer) end
end)

wm.layers:on("remove_surface", function(layer)
	for _, seat in ipairs(wm.input.seats:all()) do
		if seat:focused() == layer then focus_topmost(seat) end
	end
end)

wm.input:on("pointer_move", lock.unless_locked(function(event)
	if not wm.store.follow_mouse then return end
	if #event.pointer:buttons() > 0 then return end
	follow(event.pointer, event.position)
end))

-- A click that ends a popup grab only moves focus once it has been delivered.
wm.input:on("pointer_button", lock.unless_locked(function(event)
	if event.state == "Pressed" then M.focus_under(event.pointer) end
end))

wm.input:on("pointer_button_forwarded", lock.unless_locked(function(event)
	if event.state == "Pressed" then M.focus_under(event.pointer) end
end))

return M
