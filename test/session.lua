local M = {}

local function report(failures)
	for _, failure in ipairs(failures) do
		print("session environment: " .. failure)
	end
end

--- Tells systemd user services and D-Bus activated programs where this
--- compositor listens.
function M.export_wayland_display()
	report(wm:import_environment({ WAYLAND_DISPLAY = wm.wayland_display }))
end

--- Starts xwayland-satellite once per session. X programs reach the compositor
--- through it as ordinary Wayland clients. It prints the X display it serves,
--- which `M.spawn` hands to everything started afterwards.
function M.start_xwayland_satellite()
	if wm.is_reload then return end
	local function learn_display(line)
		local display = line:match("Connected to Xwayland on (:%d+)")
		if not display then return end
		wm.store.x11_display = display
		report(wm:import_environment({ DISPLAY = display }))
	end
	spawn("xwayland-satellite", {}, { on_stdout = learn_display, on_stderr = learn_display, survives_reload = true })
end

--- Like `spawn`, with DISPLAY set once xwayland-satellite has said where it listens.
function M.spawn(program, args, options)
	local merged = {}
	for name, value in pairs(options or {}) do merged[name] = value end
	if wm.store.x11_display then
		merged.env = { DISPLAY = wm.store.x11_display }
		for name, value in pairs((options or {}).env or {}) do merged.env[name] = value end
	end
	return spawn(program, args, merged)
end

return M
