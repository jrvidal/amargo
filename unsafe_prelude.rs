macro_rules! vec {
    ($elem:expr; $n:expr) => ({
        let vec = std::vec![$elem; $n];
        Vec::__new_from_vec(vec)
    });
    ($($x:expr),*) => ({
        let vec = std::vec![$($x),*];
        Vec::__new_from_vec(vec)
    });
    ($($x:expr,)*) => ({
      let vec = std::vec![$($x),*];
      Vec::__new_from_vec(vec)
    })
}