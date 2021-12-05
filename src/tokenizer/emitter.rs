pub trait Emitter {
    type Token;
}

pub struct DefaultEmitter {

}

impl Emitter for DefaultEmitter {
    type Token = ();
}

impl Default for DefaultEmitter {
    fn default() -> Self {
        DefaultEmitter {}
    }
}