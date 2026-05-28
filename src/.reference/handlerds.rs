


// impl DndGrabHandler for Smallvil {}
// impl WaylandDndGrabHandler for Smallvil {
//     fn dnd_requested<S: Source>(
//         &mut self,
//         source: S,
//         _icon: Option<WlSurface>,
//         seat: Seat<Self>,
//         serial: Serial,
//         type_: GrabType,
//     ) {
//         match type_ {
//             GrabType::Pointer => {
//                 let ptr = seat.get_pointer().unwrap();
//                 let start_data = ptr.grab_start_data().unwrap();

//                 // create a dnd grab to start the operation
//                 let grab = DnDGrab::new_pointer(&self.display_handle, start_data, source, seat);
//                 ptr.set_grab(self, grab, serial, Focus::Keep);
//             }
//             GrabType::Touch => {
//                 // smallvil lacks touch handling
//                 source.cancel();
//             }
//         }
//     }
// }

// //
// // Wl Output & Xdg Output
// //

// impl OutputHandler for Smallvil {}

// smithay::delegate_dispatch2!(Smallvil);
