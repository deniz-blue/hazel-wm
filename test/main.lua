print("Meowing on " .. wm.name)

wm.input.keyboard:set_layout("tr")

local drag_start_pointer = nil
local drag_start_output = nil

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

	-- Alt + R
	if e.modifiers.alt and e.keycode == 27 then
		wm.outputs:name("winit"):set_position({ x = 0, y = 0 })
		e:prevent_default()
	end

	local delta = {
		[111] = { x = 0, y = -10 }, -- Up
		[116] = { x = 0, y = 10 }, -- Down
		[113] = { x = -10, y = 0 }, -- Left
		[114] = { x = 10, y = 0 }, -- Right
	}

	if e.state == "Pressed"
		and e.modifiers.alt
		and delta[e.keycode] then
		e:prevent_default()
		print("Moving output")
		local output = wm.outputs:name("winit")
		if not output then
			print("Output not found")
			return
		end
		local pos = output:position()
		print("Current position: " .. pos.x .. ", " .. pos.y)
		output:set_position({
			x = pos.x + delta[e.keycode].x,
			y = pos.y + delta[e.keycode].y
		})
	end
end)

local last_pointer_pos = wm.input.pointer:position()
wm.input:on("pointer_move", function(e)
	local delta = {
		x = e.position.x - last_pointer_pos.x,
		y = e.position.y - last_pointer_pos.y,
	}
	last_pointer_pos = e.position
	
	print("Pointer moved by " .. delta.x .. ", " .. delta.y)

	if #wm.input.pointer:buttons() > 0 then
		local output = wm.outputs:name("winit")
		if not output then
			print("Output not found")
			return
		end
		local pos = output:position()
		local dest_pos = {
			x = pos.x - delta.x,
			y = pos.y - delta.y,
		}
		output:set_position(dest_pos)
		print("Moving output to " .. dest_pos.x .. ", " .. dest_pos.y)
		e:prevent_default()
	else
		print(".")
	end
end)

wm.input:on("pointer_button", function(e)
	if e.state == "Pressed" then
		local output = wm.outputs:name("winit")
		if not output then
			print("Output not found")
			return
		end

		drag_start_pointer = wm.input.pointer:position()
		drag_start_output = output:position()
	elseif e.state == "Released" then
		drag_start_pointer = nil
		drag_start_output = nil
	end
end)

wm.outputs:on("added", function(e)
	print("Output added " .. wm.outputs:count())
end)

-- wm.outputs.winit:move({ x = -200, y = -200 })
