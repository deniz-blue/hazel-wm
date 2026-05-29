print("Meowing on " .. wm.name)

wm:on("ready", function()
	print("WM is ready!")
	spawn("alacritty")
end)

wm.input:on("keyboard", function(e)
	print("KEY EVENT: " .. e.keycode)

	if e.keycode == 24 then
		print("You pressed 'q'!")
		e:prevent_default()
	end
end)
