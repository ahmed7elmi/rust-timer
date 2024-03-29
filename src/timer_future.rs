use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    thread,
    time::{Duration, Instant},
};

pub struct TimerFuture {
    shared_state: Arc<Mutex<SharedState>>,
}

struct SharedState {
    completed: bool,
    waker: Option<Waker>,
}

impl Future for TimerFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared_state = self.shared_state.lock().unwrap();
        if shared_state.completed {
            Poll::Ready(())
        }
        else {
            shared_state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl TimerFuture {
    pub fn new(duration: Duration, tick_duration: Duration, on_tick: fn()) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            completed: false,
            waker: None,
        }));

        let thread_shared_state = shared_state.clone();
        thread::spawn(move || {
            let now = Instant::now();
            while now.elapsed().as_secs() < duration.clone().as_secs() {
                thread::sleep(tick_duration);

                // call the callback
                on_tick();
            }

            // now we are done, let's wakeup
            let mut shared_state = thread_shared_state.lock().unwrap();
            shared_state.completed = true;

            // time for the wake up call
            if let Some(waker) = shared_state.waker.take() {
                waker.wake();
            }

        });

        TimerFuture { shared_state }
    }
}

