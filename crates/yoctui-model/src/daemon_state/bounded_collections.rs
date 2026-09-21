fn trim_front<T>(items: &mut VecDeque<T>, limit: usize) {
    while items.len() > limit {
        items.pop_front();
    }
}
