local tiling = require("tiling")
local keys = require("keys")
local floating = require("floating")
local requests = require("requests")
local focus = require("focus")
require("pointer")
local borders = require("borders")
local rules = require("rules")
local session = require("session")
local seats = require("seats")
local lock = require("lock")
local titlebars = require("titlebars")
local bar = require("bar")

tiling.exclude(floating.is_floating)
tiling.on_moved_between_outputs(requests.refit)
floating.on_change(tiling.retile)
requests.on_change(tiling.retile)
requests.on_change(borders.refresh)
wm.outputs:on("changed", tiling.retile)
wm.layers:on("mapped", tiling.retile)
wm.layers:on("changed", tiling.retile)
wm.layers:on("remove_surface", tiling.retile)
tiling.keep_hidden(requests.is_minimized)

tiling.add_workspace("4", "horizontal")
tiling.add_workspace("5", "vertical")
tiling.active("1")
tiling.set_gaps(8, 8)
borders.setup({ width = 2 })

rules.add({ app_id = "^pavucontrol$", floating = true })

floating.bind_mouse("logo", Button.Left, Button.Right)

local function configure_keyboard(keyboard)
	keyboard:set_layout("tr")
end

wm.input:on("new_keyboard", configure_keyboard)
for _, seat in ipairs(wm.input.seats:all()) do
	local keyboard = seat:keyboard()
	if keyboard then configure_keyboard(keyboard) end
end

keys.bind("super+Page_Up", function() tiling.cycle_workspace(1) end)
keys.bind("super+Page_Down", function() tiling.cycle_workspace(-1) end)

for n = 1, 5 do
	keys.bind("super+" .. n, function() tiling.active(tostring(n)) end)
	keys.bind("super+shift+" .. n, function() tiling.move_focused_to(tostring(n)) end)
end

keys.bind("super+j", function() tiling.focus_next(1) end)
keys.bind("super+k", function() tiling.focus_next(-1) end)
keys.bind("super+Return", function() tiling.swap_master() end)

local function on_focused(action)
	return function()
		local window = seats.focused_window()
		if window then action(window) end
	end
end

keys.bind("super+q", on_focused(function(window) window:close() end))
keys.bind("super+f", on_focused(requests.toggle_fullscreen))
keys.bind("super+m", on_focused(requests.toggle_maximized))
keys.bind("super+shift+space", on_focused(floating.toggle))
keys.bind("super+n", on_focused(function(window) requests.set_minimized(window, true) end))
keys.bind("super+shift+n", requests.restore_last_minimized)
keys.bind("super+shift+r", function() wm:reload() end)

keys.bind("super+shift+f", function() focus.follow_mouse(not wm.store.follow_mouse) end)

keys.bind("super+Escape", function() wm:quit() end)
keys.bind("super+l", function() session.spawn("swaylock") end)
keys.bind("super+t", function() session.spawn("alacritty") end)
keys.bind("super+w", function() session.spawn("firefox-developer-edition") end)

local function bind_always(spec, action)
	keys.bind(spec, action)
	keys.bind_locked(spec, action)
end

local function bind_media(spec, program, args)
	bind_always(spec, function() spawn(program, args) end)
end

bind_always("ctrl+super+shift+alt+q", function() wm:quit() end)

bind_media("XF86AudioRaiseVolume", "wpctl", { "set-volume", "-l", "1", "@DEFAULT_AUDIO_SINK@", "5%+" })
bind_media("XF86AudioLowerVolume", "wpctl", { "set-volume", "@DEFAULT_AUDIO_SINK@", "5%-" })
bind_media("XF86AudioMute", "wpctl", { "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle" })
bind_media("XF86AudioMicMute", "wpctl", { "set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle" })

keys.bind("super+r", function()
	tiling.retile()
	tiling.debug()
end)

session.export_wayland_display()
session.start_xwayland_satellite()
lock.setup()

wm.input:on("shortcuts_inhibit_request", function(inhibitor) inhibitor:activate() end)

tiling.restore()
