use super::blueprint::Blueprint;

pub trait Assembly {
    fn construct<B>(blueprint: B) 
        where B: Blueprint {

    }
}