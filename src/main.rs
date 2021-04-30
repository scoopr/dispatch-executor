mod timer_future;

#[link(name = "Foundation", kind = "framework")]
extern "C" {}

use {
    futures::{
        future::{BoxFuture, FutureExt},
        task::{waker_ref, ArcWake},
    },
    std::{
        future::Future,
        sync::{Arc, Mutex},
        task::{Context, Poll},
        time::Duration,
    },
    timer_future::TimerFuture,
};

use std::sync::atomic::Ordering;

use dispatch::QueuePriority;

mod objc_glue;

struct Executor {}

struct Task {
    future: Mutex<Option<BoxFuture<'static, ()>>>,
    global: bool,
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        println!("woke!");
        let task = arc_self.clone();
        if task.global {
            dispatch::Queue::global(QueuePriority::Default).exec_async(move || {
                Executor::poll_task(&task);
            });
        } else {
            dispatch::Queue::main().exec_async(move || {
                Executor::poll_task(&task);
            });
        }
    }
}

static TASK_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

impl Executor {
    fn run(&self) {
        use objc_glue::foundation::*;
        let run_loop: NSRunLoop = unsafe { NSRunLoop::mainRunLoop() };

        while TASK_COUNTER.load(Ordering::SeqCst) > 0 {
            println!("loop counter={}", TASK_COUNTER.load(Ordering::SeqCst));
            unsafe {
                // run_loop.runUntilDate_(NSDate::distantFuture());
                run_loop
                    .runMode_beforeDate_(NSString(NSDefaultRunLoopMode.0), NSDate::distantFuture());
            }
        }
    }
    fn spawn_main(future: impl Future<Output = ()> + 'static + Send) {
        TASK_COUNTER.fetch_add(1, Ordering::SeqCst);
        let future = future.boxed();
        let task = Arc::new(Task {
            future: Mutex::new(Some(future)),
            global: false,
        });
        Self::poll_task(&task);
    }
    fn spawn_global(future: impl Future<Output = ()> + 'static + Send) {
        TASK_COUNTER.fetch_add(1, Ordering::SeqCst);
        let future = future.boxed();
        let task = Arc::new(Task {
            future: Mutex::new(Some(future)),
            global: true,
        });
        dispatch::Queue::global(QueuePriority::Default).exec_async(move || {
            Self::poll_task(&task);
        });
    }

    fn poll_task(task: &Arc<Task>) {
        let mut future_slot = task.future.lock().unwrap();
        if let Some(mut future) = future_slot.take() {
            let waker = waker_ref(&task);
            let context = &mut Context::from_waker(&*waker);
            if let Poll::Pending = future.as_mut().poll(context) {
                // We're not done processing the future, so put it
                // back in its task to be run again in the future.
                *future_slot = Some(future);
            } else {
                TASK_COUNTER.fetch_sub(1, Ordering::SeqCst);
            }
        }
    }
}

fn main() {

    let executor = Executor {};
    dispatch::Queue::main().exec_async(|| {
        println!("Hello from dispatch!");
        dispatch::Queue::global(dispatch::QueuePriority::Default).exec_async(|| {
            dispatch::Queue::main().exec_after(Duration::from_secs(1), || {
                println!("Hello from late dispatch");
            });
            dispatch::Queue::main().exec_after(Duration::from_secs(3), || {
                // executor loop doesn't know if there are pending dispatch tasks
                // so the runloop exits before this is called..
                println!("oeh noe!");
            });
        });
    });

    dispatch::Queue::global(dispatch::QueuePriority::Default).exec_async(|| {
        Executor::spawn_main(async {
            println!("Hello from main, initiated from global!");
        });
    });

    Executor::spawn_global(async {
        println!("This came from elsewhere!");
    });

    Executor::spawn_main(async {
        TimerFuture::new(Duration::new(1, 0)).await;
        println!("hello from spawn1!");
    });
    Executor::spawn_main(async {
        TimerFuture::new(Duration::new(2, 0)).await;
        println!("hello from spawn2!");
    });

    executor.run();
}
