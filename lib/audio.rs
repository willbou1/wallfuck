use std::rc::Rc;
use std::cell::RefCell;
use std::ops::Deref;
use pulse::mainloop::threaded::Mainloop;
use pulse::context::{Context, introspect::ServerInfo, FlagSet as ContextFlagSet};
use pulse::stream::{Stream, FlagSet as StreamFlagSet};
use pulse::sample::{Spec, Format};
use pulse::proplist::Proplist;
use pulse::mainloop::api::Mainloop as MainloopTrait; //Needs to be in scope

mod dsp;
mod fft;
mod wav;
use dsp::{DSP,Chain, Mono, Absolute, LowPass, MovingAverage, Freq};

pub fn process_audio() {
    let spec = Spec {
        format: Format::S16NE,
        channels: 2,
        rate: 44100,
    };
    assert!(spec.is_valid());

    let mut proplist = Proplist::new().unwrap();
    proplist.set_str(pulse::proplist::properties::APPLICATION_NAME, "FooApp")
        .unwrap();

    let mut mainloop = Rc::new(RefCell::new(Mainloop::new()
        .expect("Failed to create mainloop")));

    let mut context = Rc::new(RefCell::new(Context::new_with_proplist(
        mainloop.borrow().deref(),
        "FooAppContext",
        &proplist
        ).expect("Failed to create new context")));

    // Context state change callback
    {
        let ml_ref = Rc::clone(&mainloop);
        let context_ref = Rc::clone(&context);
        context.borrow_mut().set_state_callback(Some(Box::new(move || {
            let state = unsafe { (*context_ref.as_ptr()).get_state() };
            match state {
                pulse::context::State::Ready |
                pulse::context::State::Failed |
                pulse::context::State::Terminated => {
                    unsafe { (*ml_ref.as_ptr()).signal(false); }
                },
                _ => {},
            }
        })));
    }

    context.borrow_mut().connect(None, ContextFlagSet::NOFLAGS, None)
        .expect("Failed to connect context");

    mainloop.borrow_mut().lock();
    mainloop.borrow_mut().start().expect("Failed to start mainloop");

    // Wait for context to be ready
    loop {
        match context.borrow().get_state() {
            pulse::context::State::Ready => { break; },
            pulse::context::State::Failed |
            pulse::context::State::Terminated => {
                eprintln!("Context state failed/terminated, quitting...");
                mainloop.borrow_mut().unlock();
                mainloop.borrow_mut().stop();
                return;
            },
            _ => { mainloop.borrow_mut().wait(); },
        }
    }
    context.borrow_mut().set_state_callback(None);

    let mut default_sink = Rc::new(RefCell::new(String::new()));
    let server_info_op = {
        let ds_ref = Rc::clone(&default_sink);
        context.borrow().introspect().get_server_info(move |server_info: &ServerInfo<'_>| {
            let ds = server_info.default_sink_name.as_ref().unwrap().as_ref();
            unsafe {
                (*ds_ref.as_ptr()).push_str(ds);
            }
        })
    };
    mainloop.borrow_mut().unlock();
    while server_info_op.get_state() != pulse::operation::State::Done {}
    mainloop.borrow_mut().lock();
    default_sink.borrow_mut().push_str(".monitor");

    let mut stream = Rc::new(RefCell::new(Stream::new(
        &mut context.borrow_mut(),
        "Audio Spy",
        &spec,
        None
        ).expect("Failed to create new stream")));

    // Stream state change callback
    {
        let ml_ref = Rc::clone(&mainloop);
        let stream_ref = Rc::clone(&stream);
        stream.borrow_mut().set_state_callback(Some(Box::new(move || {
            let state = unsafe { (*stream_ref.as_ptr()).get_state() };
            match state {
                pulse::stream::State::Ready |
                pulse::stream::State::Failed |
                pulse::stream::State::Terminated => {
                    unsafe { (*ml_ref.as_ptr()).signal(false); }
                },
                _ => {},
            }
        })));
    }

    stream.borrow_mut().connect_record(Some(default_sink.borrow().as_ref()),
        None, StreamFlagSet::START_CORKED).expect("Failed to connect playback");

    // Wait for stream to be ready
    loop {
        match stream.borrow().get_state() {
            pulse::stream::State::Ready => { break; },
            pulse::stream::State::Failed |
            pulse::stream::State::Terminated => {
                eprintln!("Stream state failed/terminated, quitting...");
                mainloop.borrow_mut().unlock();
                mainloop.borrow_mut().stop();
                return;
            },
            _ => { mainloop.borrow_mut().wait(); },
        }
    }
    stream.borrow_mut().set_state_callback(None);

    // create DSP chain
    let absolute = Rc::new(RefCell::new(Absolute::new()));
    let moving_average = Rc::new(RefCell::new(MovingAverage::new(100)));
    let low_pass = Rc::new(RefCell::new(LowPass::new(500 as Freq)));
    let chain: Rc<RefCell<Chain<f64>>> = Rc::new(RefCell::new(Chain::new()));
    chain.borrow_mut().push_back(absolute.clone());
    chain.borrow_mut().push_back(moving_average.clone());
    chain.borrow_mut().push_back(low_pass.clone());
    let mut mono = Mono::new(chain.clone());
    let test = mono.tick((0.5, 0.4));
    println!("Test Tick : {}", test);

    // create WAV file
    wav::write_test_wav().unwrap();

    {
        let stream_ref = Rc::clone(&stream);
        stream.borrow_mut().set_read_callback(Some(Box::new(move |nb_bytes: usize| {
            let peek_result = stream_ref.borrow_mut().peek().expect("Failed to read stream");
            match peek_result {
                pulse::stream::PeekResult::Empty => {
                    println!("empty");
                }
                pulse::stream::PeekResult::Hole(size) => {
                    println!("hole of size {}", size);
                }
                pulse::stream::PeekResult::Data(data) => {
                    println!("received {} bytes", data.len());
                }
            }
            stream_ref.borrow_mut().discard().expect("Failed to discard current fragment");
        })));
    }
    stream.borrow_mut().uncork(None);
    mainloop.borrow_mut().unlock();

    // Clean shutdown
    mainloop.borrow_mut().lock();
    stream.borrow_mut().disconnect().unwrap();
    mainloop.borrow_mut().unlock();
}
