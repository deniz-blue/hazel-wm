require("seats")
local M = {}

local MODIFIER_ALIASES = {
	super = "logo",
	logo = "logo",
	mod4 = "logo",
	ctrl = "ctrl",
	control = "ctrl",
	alt = "alt",
	shift = "shift",
}

local MODIFIER_NAMES = { "logo", "ctrl", "alt", "shift" }

local bindings = {}
local locked_bindings = {}
local swallowed_releases = {}

function M.parse(spec)
	local parts = {}
	for part in spec:gmatch("[^+]+") do
		table.insert(parts, part)
	end
	local key = table.remove(parts)
	if not key then
		error("empty key binding: " .. spec)
	end

	local modifiers = {}
	for _, name in ipairs(parts) do
		local modifier = MODIFIER_ALIASES[name:lower()]
		if not modifier then
			error("unknown modifier '" .. name .. "' in key binding: " .. spec)
		end
		modifiers[modifier] = true
	end
	return { key = key, modifiers = modifiers }
end

local function modifiers_match(wanted, held)
	for _, name in ipairs(MODIFIER_NAMES) do
		if (wanted[name] or false) ~= (held[name] or false) then
			return false
		end
	end
	return true
end

function M.matches(binding, event)
	return event.state == "Pressed"
		and event.raw_key == Key[binding.key]
		and modifiers_match(binding.modifiers, event.modifiers)
end

function M.bind(spec, action)
	local binding = M.parse(spec)
	binding.action = action
	table.insert(bindings, binding)
end

function M.bind_locked(spec, action)
	local binding = M.parse(spec)
	binding.action = action
	table.insert(locked_bindings, binding)
end

function M.clear()
	bindings = {}
	locked_bindings = {}
	swallowed_releases = {}
end

local function dispatch(list, event)
	if event.state == "Released" and swallowed_releases[event.keycode] then
		swallowed_releases[event.keycode] = nil
		return true
	end
	if event.shortcuts_inhibited then return false end
	for _, binding in ipairs(list) do
		if M.matches(binding, event) then
			swallowed_releases[event.keycode] = true
			binding.action()
			return true
		end
	end
	return false
end

function M.handle(event) return dispatch(bindings, event) end

function M.handle_locked(event) return dispatch(locked_bindings, event) end

wm.input:on("key", function(event)
	local handle = wm.is_session_locked and M.handle_locked or M.handle
	if handle(event) then
		event:prevent_default()
	end
end)

return M
