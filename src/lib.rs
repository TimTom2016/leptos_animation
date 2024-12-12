use instant::Instant;
use leptos::prelude::*;
use std::cmp::PartialEq;
use std::fmt::Debug;
use std::ops::{Add, Deref, Mul};
use std::{collections::VecDeque, ops::Sub, time::Duration};

pub mod animation_target;
pub mod easing;

#[derive(Clone)]
enum AnimationContextState {
    NoAnimationFrameRequested,
    AnimationFrameRequested(AnimationFrameRequestHandle),
    CustomAnimationFrameRequested,
}

/// The `AnimationContext` handles updating all animated values and calls to `window.request_animation_frame()`.
/// It is required to provide one in a parent context before calling [`create_animated_signal()`]
/// ```
/// # use leptos::prelude::*;
/// # use leptos_animation::AnimationContext;
/// # let owner = Owner::new();
/// # owner.set();
///  AnimationContext::provide();
/// # owner.unset();
/// ```
#[derive(Copy, Clone)]
pub struct AnimationContext {
    /// The `animation_frame` trigger is the root for all animation updates. It is triggered on
    /// the `window.request_animation_frame()` callback. It is not necessary to notify or track
    /// this trigger yourself, it will happen automatically when animated signals exist.
    pub animation_frame: Trigger,
    state: StoredValue<AnimationContextState>,
    custom_request_animation_frame: StoredValue<Option<Box<dyn Fn()>>, LocalStorage>,
}

impl AnimationContext {
    /// Sets up an AnimationContext for this scope and all child scopes. For normal use you only
    /// need to call this once in a root component of the application.
    pub fn provide() -> Self {
        let animation_frame = Trigger::new();
        let state = StoredValue::new(AnimationContextState::NoAnimationFrameRequested);

        let animation_context = AnimationContext {
            animation_frame,
            state,
            custom_request_animation_frame: StoredValue::new_local(None),
        };
        provide_context(animation_context);

        on_cleanup(move || {
            if let AnimationContextState::AnimationFrameRequested(handle) = state.get_value() {
                handle.cancel()
            }
        });

        animation_context
    }

    /// This method can be used instead of `provide` when you are in a non-web environment such as
    /// a desktop application. *For web environments it is recommended to use the normal `provide` instead*
    ///
    /// There are two extra callbacks that have to be correctly called and implemented in order
    /// for this library to correctly function.
    ///
    /// The callback given in the argument has to call some function that triggers an animation frame
    /// request. For example, in the `winit` crate this would be calling [`Window::request_redraw()`](https://docs.rs/winit/latest/winit/window/struct.Window.html#method.request_redraw).
    /// This callback will be called at most once per animation frame.
    ///
    /// The callback returned from this function should be called when the animation frame from the
    /// previous callback has arrived.
    /// For example, in the `winit` crate this should be called when the [`WindowEvent::RedrawRequested`](https://docs.rs/winit/latest/winit/event/enum.WindowEvent.html#variant.RedrawRequested) event happens
    /// Extraneous calls to this callback are ignored.
    ///
    /// ````
    /// # // Lots of boilerplate to simulate winit environment
    /// # use leptos::prelude::Owner;
    /// # struct Window {}
    /// # impl Window { fn request_redraw(&self) {} }
    /// # let window = Window {};
    /// # let owner = Owner::new();
    /// # owner.set();
    /// # struct EventLoop {}
    /// # impl EventLoop { fn run(&self, f: impl Fn(Event, ())) {} }
    /// # let event_loop = EventLoop {};
    /// # enum WindowEvent { RedrawRequested }
    /// # enum Event { WindowEvent { event: WindowEvent}, Other }
    /// use leptos_animation::AnimationContext;
    ///
    /// let (_, on_redraw_requested) =
    ///         AnimationContext::provide_with_custom_request_animation_frame(move || {
    ///             window.request_redraw();
    ///         });
    ///
    /// event_loop.run(move |event, elwt| match event {
    ///         Event::WindowEvent {
    ///             event: WindowEvent::RedrawRequested,
    ///             ..
    ///         } => on_redraw_requested(),
    ///         _ => {}
    /// });
    /// # owner.unset();
    /// ````
    pub fn provide_with_custom_request_animation_frame(
        callback: impl Fn() + 'static,
    ) -> (Self, impl Fn()) {
        let animation_context = Self::provide();
        animation_context
            .custom_request_animation_frame
            .set_value(Some(Box::new(callback)));

        (animation_context, move || {
            if !matches!(
                animation_context.state.get_value(),
                AnimationContextState::NoAnimationFrameRequested
            ) {
                animation_context
                    .state
                    .set_value(AnimationContextState::NoAnimationFrameRequested);
                animation_context.animation_frame.notify();
            }
        })
    }

    /// Manually request a new animation frame. It will result in a `notify()` on the
    /// `AnimationContext.animation_frame` trigger which updates all running animations
    /// simultaneously. Repeated calls will result in only a single animation frame request.
    ///
    /// Animated signals will call this automatically when they are running, it is not necessary
    /// to call this function unless you are doing something custom.
    pub fn request_animation_frame(&self) {
        // Prevent multiple animation frame requests from existing simultaneously
        if matches!(
            self.state.get_value(),
            AnimationContextState::NoAnimationFrameRequested
        ) {
            self.custom_request_animation_frame
                .with_value(
                    |custom_request_animation_frame| match custom_request_animation_frame {
                        None => {
                            let this = *self;
                            self.state
                                .set_value(AnimationContextState::AnimationFrameRequested(
                                    request_animation_frame_with_handle(move || {
                                        this.state.set_value(
                                            AnimationContextState::NoAnimationFrameRequested,
                                        );
                                        this.animation_frame.notify();
                                    })
                                    .unwrap(),
                                ))
                        }
                        Some(callback) => {
                            self.state
                                .set_value(AnimationContextState::CustomAnimationFrameRequested);
                            callback()
                        }
                    },
                );
        }
    }
}

/// An `AnimationTarget` is a target value for the animation system to ease towards to along with
/// details about the animation such as its duration, easing method and how to deal with previous animations.
///
/// An AnimationTarget can also be created from a tuple:
/// ```
/// # use std::time::Duration;
/// # use leptos_animation::{AnimationMode, AnimationTarget, easing};
/// let _: AnimationTarget<u32> = (42, Duration::from_secs_f64(1.5), easing::ELASTIC_IN, AnimationMode::ReplaceOrStart).into();
/// ```
///
/// It is possible to omit any combination of duration, easing or animation mode:
/// ```
/// # use std::time::Duration;
/// # use leptos_animation::AnimationTarget;
/// // Omit easing & animation mode, will be filled in by default values
/// let _: AnimationTarget<u32> = (42, Duration::from_secs_f64(1.5)).into();
/// ```
///
/// If you want to use all the default animation options you can call `into()` directly on a target value:
/// ```
/// # use std::time::Duration;
/// # use leptos_animation::AnimationTarget;
/// let _: AnimationTarget<u32> = 42.into();
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnimationTarget<T> {
    /// The final value to animate towards to
    pub target: T,

    /// The time for which the animation plays. Defaults to 0.5 seconds
    pub duration: Duration,

    /// The easing method to apply during the animation. Defaults to [`SINE_OUT`](easing::SINE_OUT)
    pub easing: Easing,

    /// The mode specifies how to deal with running animation. Defaults to [`Start`](AnimationMode::Start).
    /// This can be used to add, overwrite or cancel running animations.
    /// See [`AnimationMode`] for more information
    pub mode: AnimationMode,
}

/// The `AnimationMode` specifies how to handle new animation target values with respect to currently running animations
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnimationMode {
    /// Always start a new animation on top of the already running animations when the input signal changes.
    /// This is the default mode. For 'bursty' input signals which can update many times in quick succession (like mouse move events)
    /// it is recommended to use one of the other modes to prevent many overlapping animations running simultaneously
    Start,

    /// Replace the target value of the latest running animation or start a new animation if there are no animations running
    ReplaceOrStart,

    /// Replace the target of the latest running animation or snap directly to the target if there are no animations running
    ReplaceOrSnap,

    /// Cancels any previous animation and sets the output directly to the target value
    Snap,
}

/// An easing function is one that takes a value between 0.0 - 1.0 and maps it to another value between 0.0 and 1.0
/// See `https://easings.net` for a list of implemented functions
pub type Easing = fn(f64) -> f64;

struct Animation<T, I> {
    from: T,
    to: T,
    to_i: I,
    start: Instant,
    duration: Duration,
    easing: Easing,
}

impl<T, I> Animation<T, I> {
    fn is_finished(&self) -> bool {
        Instant::now() > self.start + self.duration
    }

    fn progress(&self) -> f64 {
        (self.easing)((Instant::now() - self.start).as_secs_f64() / self.duration.as_secs_f64())
    }
}

enum AnimationStatus<T, I> {
    /// No animation running
    Static(T),

    /// No animation running, but animated signal is expected to update in the next animation frame to this value.
    /// After that it will revert back to Static
    Snap(T),

    /// Animations are running
    /// The `VecDeque` is guaranteed to contain at least one animation. All animations are guaranteed
    /// to be sorted in reverse order of when they started with the most recent one in front and
    /// the oldest one in the back.
    Running {
        to: T,
        to_i: I,
        animations: VecDeque<Animation<T, I>>,
    },
}

impl<T: Clone, I> AnimationStatus<T, I> {
    fn remove_finished_animations(&mut self) {
        match self {
            AnimationStatus::Static(_) => {}
            AnimationStatus::Snap(value) => *self = AnimationStatus::Static(value.clone()),
            AnimationStatus::Running { to, animations, .. } => {
                animations.retain(|animation| !animation.is_finished());
                if animations.is_empty() {
                    *self = AnimationStatus::Snap(to.clone());
                }
            }
        }
    }
}

// This is used to filter signals with create_memo. Yes, a total hack.
#[derive(Clone)]
enum SignalUpdate {
    Ignore,
    Update,
}

impl PartialEq for SignalUpdate {
    fn eq(&self, other: &Self) -> bool {
        match other {
            SignalUpdate::Ignore => true,
            SignalUpdate::Update => false,
        }
    }
}

/// Default linear tween between any type of number
pub fn tween_default<T, I>(from: &T, to: &T, progress: f64) -> I
where
    T: Copy,
    T: Sub<T, Output = I>,
    I: Mul<f64, Output = I>,
    I: Add<T, Output = I>,
{
    (*to - *from) * progress + *from
}

#[derive(Clone, Copy)]
pub struct AnimatedSignal<T: 'static, I: 'static + Send + Sync> {
    animation_status: StoredValue<AnimationStatus<T, I>>,
    update_animation_status_effect: Effect<LocalStorage>,
    animation_tick: Memo<SignalUpdate>,
    animated_signal: Signal<I>,
}

impl<T, I: Send + Sync> Deref for AnimatedSignal<T, I> {
    type Target = Signal<I>;

    fn deref(&self) -> &Self::Target {
        &self.animated_signal
    }
}

impl<T, I: Send + Sync> Dispose for AnimatedSignal<T, I> {
    fn dispose(self) {
        self.animation_status.dispose();
        self.animation_tick.dispose();
        self.update_animation_status_effect.dispose();
        self.animated_signal.dispose();
    }
}

impl<T, I: Send + Sync> AnimatedSignal<T, I> {
    /// Create a derived signal that animated the value of the input signals.
    /// Takes as input a reactive source callback function and a tween function.
    ///
    /// The source callback function is run in a reactive context and is expected to take the value of one or more input
    /// signals and return an `AnimationTarget` value. An `AnimationTarget` specifies a target value to
    /// animate towards and details about the duration, easing and animation of how to animate towards it.
    /// There are shortcut methods to create an `AnimationTarget` with default values, see
    /// [`AnimationTarget`] for details.
    ///
    /// The tween callback specifies how to interpolate between two input values. As input it takes three
    /// arguments: `from`, `to` and `progress`. Where `from` and `to` are the values from the input signal
    /// and the `progress` is a value between 0.0 - 1.0. The easing is already applied to the `progress`.
    /// The tween function is expected to do a linear interpolation between `from` & `to` and return the
    /// result.
    ///
    /// If the input is in any way numeric or supports the `Add`, `Sub` and `Mul<f64>` traits it is recommended
    /// to use the [`tween_default`] function as input which performs a simple `(to - from) * progress + from`.
    ///
    /// If you are dealing with structs that are composed of numbers (for example a `Position { x: f64, y: f64 }`)
    /// you can use the [derive_more](https://docs.rs/crate/derive_more/latest) crate to implement the necessary traits.
    /// This way you can still use the `tween_default` function.
    ///
    /// This function is generic over two types: `T` and `I`.
    /// * `T` is the type of values that are animated between. Animations are always from a `T` towards another `T`
    /// * `I` is the type of the interpolated values between values of type `T`.
    ///
    /// In simple cases `I` is the same as `T` such as animating between `f64`'s. But they can also be different
    /// if for example the `T` is an enum which cannot represent 'in-between' values by itself.
    ///
    /// Updates to the derived signal only happen on browser animation frames and only when there are animations
    /// running. If you are dealing with a HTML Canvas it is recommended to use a `create_effect()` to draw on the
    /// canvas and subscribe directly to the animated signals.
    /// All animated signals update simultaneously on animation frames so even if you subscribe to multiple animated
    /// input signals the effect will never run more than 60fps.
    ///
    /// # Additive animations
    ///
    /// This library uses an additive animation system. This means that multiple animations with different
    /// targets and different durations can play simultaneously without them interrupting each other.
    ///
    /// Internally all animations are towards 0. For example if we start an animation from 0 to 100, this is
    /// converted to an animation from -100 to 0 which gets added to the final 100 value.
    ///
    /// If then a second animation is started from 100 to 1000 it gets converted to an animation from -900 to 0.
    /// Both the -100 to 0 and the -900 to 0 animation value get added to the final 1000 value until both settle on 1000 as they reach 0.
    ///
    /// This allows for all animations to play to completion even if animations are started before the previous animation is finished.
    ///
    /// # Examples
    /// ```
    /// # use std::time::Duration;
    /// # use leptos::prelude::*;
    /// # use leptos_animation::{AnimationContext, AnimationMode, AnimationTarget, easing, tween_default, AnimatedSignal};
    /// # let owner = Owner::new();
    /// # owner.set();
    /// # AnimationContext::provide();
    /// let (value, set_value) = signal(42.0);
    ///
    /// // Simple default animation
    /// let animated_value = AnimatedSignal::new(move || value.get().into(), tween_default);
    ///
    /// // Custom duration
    /// let slow_value = AnimatedSignal::new(move || (value.get(), Duration::from_secs_f64(5.0)).into(), tween_default::<f64, f64>);
    ///
    /// // Custom duration, easing & mode
    /// let custom_value = AnimatedSignal::new(
    ///         move || AnimationTarget {
    ///             target: value.get(),
    ///             duration: Duration::from_secs_f64(1.5),
    ///             easing: easing::ELASTIC_IN_OUT,
    ///             mode: AnimationMode::ReplaceOrStart
    ///         },
    ///         tween_default);
    ///
    /// // Custom tween function
    /// let tween_value = AnimatedSignal::new(
    ///         move || value.get().into(),
    ///         |from, to, progress| {
    ///             (to - from) * progress + from
    ///         });
    ///
    /// # owner.unset();
    /// ```
    pub fn new(
        source: impl Fn() -> AnimationTarget<T> + 'static + Send + Sync,
        tween: fn(&T, &T, f64) -> I,
    ) -> AnimatedSignal<T, I>
    where
        T: Clone,
        I: Clone,
        I: Sub<I, Output = I>,
        T: Send + Sync + 'static,
        I: Send + Sync + 'static,
    {
        let context: AnimationContext = use_context().expect(
            "No AnimationContext present, call AnimationContext::provide() in a parent scope",
        );

        let source = Signal::derive(source);

        let animation_status = StoredValue::new(AnimationStatus::<T, I>::Static(
            source.get_untracked().target,
        ));

        // Effect that listens to changes in the source and updates the animation status
        let update_animation_status_effect = Effect::new(move |prev: Option<()>| {
            let animation_target = source.get();

            // Don't start an animation the very first run
            if prev.is_none() {
                return;
            }

            animation_status.update_value(|animation_status| {
                match animation_status {
                    // Starting an animation from a non-running state
                    AnimationStatus::Static(state) | AnimationStatus::Snap(state) => {
                        match animation_target.mode {
                            AnimationMode::Start | AnimationMode::ReplaceOrStart => {
                                let to_i =
                                    tween(&animation_target.target, &animation_target.target, 1.0);
                                *animation_status = AnimationStatus::Running {
                                    to: animation_target.target.clone(),
                                    to_i: to_i.clone(),
                                    animations: VecDeque::from([Animation {
                                        from: state.clone(),
                                        to: animation_target.target,
                                        to_i,
                                        start: Instant::now(),
                                        duration: animation_target.duration,
                                        easing: animation_target.easing,
                                    }]),
                                }
                            }
                            AnimationMode::ReplaceOrSnap | AnimationMode::Snap => {
                                *animation_status = AnimationStatus::Snap(animation_target.target)
                            }
                        }
                    }
                    // Start an animation from a running state
                    AnimationStatus::Running {
                        to,
                        to_i,
                        animations,
                    } => match animation_target.mode {
                        AnimationMode::Start => {
                            let new_to_i =
                                tween(&animation_target.target, &animation_target.target, 1.0);

                            animations.push_front(Animation {
                                from: to.clone(),
                                to: animation_target.target.clone(),
                                to_i: new_to_i.clone(),
                                start: Instant::now(),
                                duration: animation_target.duration,
                                easing: animation_target.easing,
                            });
                            *to = animation_target.target;
                            *to_i = new_to_i;
                        }
                        // This arm can only be reached when there are still live animations, so we perform the 'replace' operation
                        AnimationMode::ReplaceOrStart | AnimationMode::ReplaceOrSnap => {
                            *to = animation_target.target.clone();
                            *to_i = tween(&animation_target.target, &animation_target.target, 1.0);
                            let last_animation = animations.front_mut().unwrap();
                            last_animation.to = animation_target.target;
                            last_animation.to_i = to_i.clone();
                        }
                        AnimationMode::Snap => {
                            *animation_status = AnimationStatus::Snap(animation_target.target)
                        }
                    },
                }
            });
            context.request_animation_frame();
        });

        // Signal that derives from the global animation_frame signal but only
        // fires when 'this' animation has something to update.
        let animation_tick = Memo::new(move |_| {
            context.animation_frame.track();

            let was_snap = animation_status.with_value(|animation_status| {
                matches!(animation_status, AnimationStatus::Snap(_))
            });

            animation_status.update_value(|animation_status| {
                animation_status.remove_finished_animations();
            });

            if was_snap {
                SignalUpdate::Update
            } else {
                animation_status.with_value(|animation_status| match animation_status {
                    AnimationStatus::Static(_) => SignalUpdate::Ignore,
                    _ => SignalUpdate::Update,
                })
            }
        });

        let animated_signal = Signal::derive(move || {
            let _ = animation_tick.get();

            let i: I = animation_status.with_value(|animation_status| match animation_status {
                AnimationStatus::Static(state) | AnimationStatus::Snap(state) => {
                    tween(state, state, 1.0)
                }
                AnimationStatus::Running {
                    animations, to_i, ..
                } => {
                    // Keep this signal updated in the animation loop
                    context.request_animation_frame();

                    // Add all animation results to a single value
                    animations.iter().fold(to_i.clone(), |acc, animation| {
                        let animation_value =
                            tween(&animation.from, &animation.to, animation.progress());

                        acc - (animation.to_i.clone() - animation_value)
                    })
                }
            });
            i
        });

        AnimatedSignal {
            animation_status,
            update_animation_status_effect,
            animation_tick,
            animated_signal,
        }
    }
}
