local M = {}

--- The lock screen gets the input while the session is locked or locking.
--- Handlers wrapped with this stay out of the way; a handler that should
--- still act (volume keys) is registered without it and decides itself.
function M.unless_locked(handler)
	return function(...)
		if wm.is_session_locked then return end
		return handler(...)
	end
end

function M.setup()
	wm.session_lock:set_backdrop_color("#000000")
end

return M
