use futures::{
    future::{BoxFuture, FutureExt},
    task::{waker_ref, ArcWake},
};
use std::{
    future::Future,
    sync::mpsc::{sync_channel, Receiver, SyncSender},
    sync::{Arc, Mutex},
    task::Context,
    time::Duration,
};

mod timer_future;
// The timer we wrote in the previous section:
use timer_future::TimerFuture;

// Executor
struct Executor {
    ready_queue: Receiver<Arc<Task>>
}

impl Executor {
    fn run(&self) {
        while let Ok(task) = self.ready_queue.recv() { // todo: Should we do some error handling here? Check on Err??
            let mut future_slot = task.future.lock().unwrap();
            if let Some(mut future) = future_slot.take() {
                // get the waker from the task
                let waker = waker_ref(&task);
                let context = &mut Context::from_waker(&waker);

                if future.as_mut().poll(context).is_pending() {
                    *future_slot = Some(future);
                }

            }
        }
    }
}

// Spawner
#[derive(Clone)]
struct Spawner {
    task_sender: SyncSender<Arc<Task>>
}

impl Spawner {
    fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let future = future.boxed();
        let task = Arc::new(Task {
            future: Mutex::new(Some(future)),
            task_sender: self.task_sender.clone(),
        });
        self.task_sender.send(task).expect("too many tasks queued");
    }
}

// Task
struct Task {
    future: Mutex<Option<BoxFuture<'static, ()>>>, // try to remove the mutex and see what will happen!!
    task_sender: SyncSender<Arc<Task>>,
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let cloned = arc_self.clone();
        arc_self
            .task_sender
            .send(cloned)
            .expect("too many tasks queued");
    }
}

fn new_executor_and_spawner() -> (Executor, Spawner) {
    const MAX_QUEUED_TASKS: usize = 10_000;
    let (task_sender, ready_queue) = sync_channel(MAX_QUEUED_TASKS);
    (Executor { ready_queue }, Spawner { task_sender })
}

fn main() {
    let (executor, spawner) = new_executor_and_spawner();

    // Spawn another task
    spawner.spawn(async {
        TimerFuture::new(Duration::from_secs(20), Duration::from_secs(2), || {
            println!("tick!");
        }).await;

    });

    drop(spawner);

    executor.run();
}
