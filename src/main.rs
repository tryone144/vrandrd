/*
 * This file is part of the *vrandrd* application.
 *
 * (c) 2018 Bernd Busse
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate xcb;

use xcb::xproto;
use xcb::randr;


fn main() {
    // Connect to X screen
    let (conn, screen_num) = xcb::Connection::connect(None).unwrap();
    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).unwrap();

    println!(" :: Connected to screen {:?}", screen_num);

    // Create dummy window
    let dummy: xcb::Window = conn.generate_id();
    xproto::create_window(&conn, 0, dummy, screen.root(), 0, 0, 1, 1, 0, 0, 0, &[]);
    conn.flush();

    // Get 'hotplug_mode_update' atom
    let cookie = xproto::intern_atom(&conn, false, "hotplug_mode_update");
    let atom = cookie.get_reply().unwrap().atom();

    let mut supported_outputs: Vec<randr::Output> = Vec::new();

    // Get all outputs for this screen
    let cookie = randr::get_screen_resources(&conn, dummy);
    let screen_res = cookie.get_reply().unwrap();
    println!(" :: Found {} outputs", screen_res.num_outputs());

    // Iterate over all outputs
    let timestamp: xcb::Timestamp = conn.generate_id();
    for output in screen_res.outputs() {
        let cookie = randr::get_output_info(&conn, *output, timestamp);
        let info = cookie.get_reply().unwrap();

        let cookie = randr::query_output_property(&conn, *output, atom);
        let reply = cookie.get_reply();
        match reply {
            Ok(_) => {
                println!("    | {} supports 'hotplug_mode_update'", String::from_utf8_lossy(info.name()));
                supported_outputs.push(*output);
            },
            Err(err) => {
                if err.error_code() != 15 { // NOT "property not found"
                    panic!(err);
                }
            }
        };
    }

    if supported_outputs.len() == 0 {
        panic!("No suitable outputs found!");
    }

    // RandR extension notify events
    let randr_ev_base = conn.get_extension_data(&mut randr::id()).unwrap().first_event();
    let cookie = randr::select_input(&conn, screen.root(), randr::NOTIFY_MASK_OUTPUT_CHANGE as u16);
    let _ = cookie.request_check();

    // Wait for Xserver events
    loop {
        conn.flush();
        let event = conn.wait_for_event().unwrap();

        if event.response_type() == randr_ev_base + randr::NOTIFY {
            let ev: &randr::NotifyEvent = unsafe { xcb::cast_event(&event) };
            if ev.sub_code() == randr::NOTIFY_OUTPUT_CHANGE as u8 {
                let data = ev.u().oc();

                let cookie = randr::get_output_info(&conn, data.output(), timestamp);
                let info = cookie.get_reply().unwrap();
                println!("Received output changed event for {}", String::from_utf8_lossy(info.name()));
            }
        }
    }
}
