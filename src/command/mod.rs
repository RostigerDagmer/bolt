pub mod pool_manager;

pub use pool_manager::CommandManager;

// if !app.await.window.is_minimized().await {
                
//     if dirty_swapchain {
//         app.await.recreate_swapchain();
//         dirty_swapchain = false;
//     }

//     match event {
//         Event::WindowEvent { event, .. } => {
//             match event {
//                 WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
//                 WindowEvent::KeyboardInput { input, .. } => {
//                     if input.state == ElementState::Pressed {
//                         if input.virtual_keycode == Some(VirtualKeyCode::Q)
//                             && (modifiers.ctrl() || modifiers.logo())
//                         {
//                             *control_flow = ControlFlow::Exit;
//                         }
//                     }
//                 }
//                 WindowEvent::MouseInput { .. } => {}
//                 WindowEvent::ModifiersChanged(m) => modifiers = m,
//                 _ => (),
//             }
//             match builder.window_event {
//                 Some(event_fn) => {
//                     event_fn(&mut app, &mut app_data, &event);
//                 }
//                 None => {}
//             }
//         }
//         Event::MainEventsCleared => {
//             let now = now.elapsed().unwrap();
//             if app.elapsed_ticks % 10 == 0 {
//                 let cpu_time = now.as_millis() as f32 - app.elapsed_time.as_millis() as f32;
//                 let title = format!("{} | cpu:{:.1} ms, gpu:{:.1} ms", app.settings.name, cpu_time, app.renderer.gpu_frame_time);
//                 app.window.set_title(&title);
//             }
//             app.elapsed_time = now;

//             match builder.update {
//                 Some(update_fn) => {
//                     update_fn(&mut app, &mut app_data);
//                 }
//                 None => {}
//             }

//             dirty_swapchain = match builder.render {
//                 Some(render_fn) => {
//                     let future = render_fn(&mut app, &mut app_data);
//                     pin!(future);
//                     let res = runtime.block_on(future);
//                     matches!(
//                         res,
//                         Err(AppRenderError::DirtySwapchain)
//                     )
//                 }
//                 None => false,
//             };

//             app.elapsed_ticks += 1;
//         }
//         Event::Suspended => println!("Suspended."),
//         Event::Resumed => println!("Resumed."),
//         Event::LoopDestroyed => unsafe {
//             app.renderer.context.device().device_wait_idle().unwrap();
//         },
//         _ => {}
//     }
// }