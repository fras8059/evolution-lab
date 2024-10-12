pub fn init() {
    env_logger::init();
}

#[cfg(test)]
mod tests {
    use std::ptr;

    use crate::config::log::init;

    #[test]
    fn test_init() {
        let noop_logger = log::logger();
        init();
        let logger = log::logger();
        assert!(
            !ptr::eq(&*noop_logger, &*logger),
            "Should initialize global logger"
        );
    }
}
