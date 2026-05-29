print("Meowing on " .. wm.name)

wm:on("ready", function()
	print("WM is ready!")
	spawn("alacritty")
end)

---@param e KeyboardEvent
wm.input:on("keyboard", function(e)
	print("KEY EVENT: " .. e.keycode)

	-- Alt + T
	if e.modifiers.alt and e.keycode == 28 then
		print("Alt + T pressed")
		spawn("alacritty")
		e:prevent_default()
	end
end)

wm.outputs:on("added", function(e)
	print("Output added " .. #wm.outputs)

	for k, v in pairs(wm.outputs) do
		print(k)
	end
end)

-- wm.outputs.winit:move({ x = -200, y = -200 })

