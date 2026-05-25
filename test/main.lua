print("meow")

input:on("key", function (data)
	print("key event: " .. data)
end)
