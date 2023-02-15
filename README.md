# tetrii
Project to learn about Rust

Every new language I try to learn I start with tetrii, multiple board tetris. I've written it in C with X-windows, C++, Java, 
Javascript, Python, and ELisp, assuming I haven't forgotten any others.

Rust is, unfortunately, giving me more trouble than most of the others. The variable ownership is a problem - either I still
need to learn some tricks about how to do it, or my overall design is unsuitable to Rust. I'd appreciate any tips as to whether
fixing this will require a redesign, or whether there is a simple idea I'm missing to access what I need.

This version ran successfully with multiple boards. That was probably not good either - the boards popped up in different windows
and I needed to use a static Vec to hold the values. That version did not have the file controller.rs, but main popped up the 
windows directly.

Controller is intended to be a master window - it will allow choosing the number of boards, their sizes and configurations, and
report on overall score. It also is intended to hold copies of the Board objects, in a Vec<Rc<RefCell<Board>>>. The Controller
window comes up with its buttons, but when I try to start the boards I get the error

```
thread 'main' panicked at 'already borrowed: BorrowMutError', src/controller.rs:134:56
stack backtrace:
...
  18:        0x10be2aecb - tetrii::controller::Controller::build_ui::{{closure}}::hea2e16f9c861c0eb
                               at /Users/russell/personal/projects/tetrii/rust/tetrii/src/controller.rs:134:51
...
  53:        0x10be4402d - tetrii::main::h789dd27ee9c6b980
                               at /Users/russell/personal/projects/tetrii/rust/tetrii/src/main.rs:24:5

```
So it appears (I could be wrong) the problem is the immutable borrow of Controller in main() means I can't borrow app from
it mutable to start up the boards. I've tried all sorts of wrapping things in RefCell's and Rc's, but whatever I've tried has
not worked. One thought I've had is making one big window rather than lots of them that can be moved around - but somehow I fear
after spending all the time to do that I'd run into the same problem - and anyway, this design should be possible (sholdn't it?)
