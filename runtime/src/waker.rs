use std::mem::ManuallyDrop;
use std::sync::Arc;
use std::task::{RawWaker, RawWakerVTable, Waker};

pub fn waker_fn<F: Fn() + Send + Sync + 'static>(f: F) -> Waker {
    let raw = Arc::into_raw(Arc::new(f)) as *const ();
    let vtable = &WakerHelper::<F>::VTABLE;
    unsafe { Waker::from_raw(RawWaker::new(raw, vtable)) }
}

struct WakerHelper<F>(F);

impl<F: Fn() + 'static> WakerHelper<F> {
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        Self::clone,
        Self::wake,
        Self::wake_by_ref,
        Self::drop,
    );

    unsafe fn clone(this: *const ()) -> RawWaker {
        let arc = ManuallyDrop::new(Arc::from_raw(this as *const F));
        std::mem::forget(arc.clone());
        RawWaker::new(this, &Self::VTABLE)
    }
    unsafe fn wake(this: *const ()) {
        let arc = Arc::from_raw(this as *const F);
        (arc)();
    }
    unsafe fn wake_by_ref(this: *const ()) {
        let arc = ManuallyDrop::new(Arc::from_raw(this as *const F));
        (arc)();
    }
    unsafe fn drop(this: *const ()) {
        drop(Arc::from_raw(this as *const F));
    }
}
