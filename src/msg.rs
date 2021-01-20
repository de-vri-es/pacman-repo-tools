#[doc(hidden)]
pub use yansi::Paint;

#[doc(hidden)]
pub fn use_color() -> bool {
	use std::sync::atomic::{AtomicU32, Ordering};
	static SHOULD_COLOR: AtomicU32 = AtomicU32::new(0);
	const COLOR_YES: u32 = 1;
	const COLOR_NO: u32 = 2;

	match SHOULD_COLOR.load(Ordering::Relaxed) {
		COLOR_YES => return true,
		COLOR_NO => return false,
		_ => (),
	};

	if std::env::var_os("CLICOLOR").map(|x| x == "0") == Some(true) {
		SHOULD_COLOR.store(COLOR_NO, Ordering::Relaxed);
		false
	} else if std::env::var_os("CLICOLOR_FORCE").map(|x| x == "1") == Some(true) {
		SHOULD_COLOR.store(COLOR_YES, Ordering::Relaxed);
		true
	} else if atty::is(atty::Stream::Stdout) {
		SHOULD_COLOR.store(COLOR_YES, Ordering::Relaxed);
		true
	} else {
		SHOULD_COLOR.store(COLOR_NO, Ordering::Relaxed);
		false
	}
}

#[macro_export]
#[rustfmt::skip]
macro_rules! plain {
	($($args:tt)*) => {
		println!("    {}", format_args!($($args)*))
	}
}

#[macro_export]
#[rustfmt::skip]
macro_rules! msg {
	($($args:tt)*) => {
		println!(
			"{} {}",
			$crate::msg::Paint::green("==>").bold(),
			$crate::msg::Paint::default(format_args!($($args)*)).bold(),
		)
	}
}

#[macro_export]
#[rustfmt::skip]
macro_rules! msg2 {
	($($args:tt)*) => {
		println!(
			" {} {}",
			$crate::msg::Paint::blue(" ->").bold(),
			$crate::msg::Paint::default(format_args!($($args)*)).bold(),
		)
	}
}

#[macro_export]
#[rustfmt::skip]
macro_rules! warning {
	($($args:tt)*) => {
		println!(
			"{} {}",
			$crate::msg::Paint::yellow("==> WARNING:").bold(),
			$crate::msg::Paint::default(format_args!($($args)*)).bold(),
		)
	}
}

#[macro_export]
#[rustfmt::skip]
macro_rules! error {
	($($args:tt)*) => {
		println!(
			"{} {}",
			$crate::msg::Paint::red("==> ERROR:").bold(),
			$crate::msg::Paint::default(format_args!($($args)*)).bold(),
		)
	}
}

#[macro_export]
#[rustfmt::skip]
macro_rules! plain_no_eol {
	($($args:tt)*) => {
		print!("    {}", format_args!($($args)*))
	}
}

#[macro_export]
#[rustfmt::skip]
macro_rules! msg_no_eol {
	($($args:tt)*) => {
		print!(
			"{} {}",
			$crate::msg::Paint::green("==>").bold(),
			$crate::msg::Paint::default(format_args!($($args)*)).bold(),
		)
	}
}

#[macro_export]
#[rustfmt::skip]
macro_rules! msg2_no_eol {
	($($args:tt)*) => {
		print!(
			" {} {}",
			$crate::msg::Paint::blue(" ->").bold(),
			$crate::msg::Paint::default(format_args!($($args)*)).bold(),
		)
	}
}

#[macro_export]
#[rustfmt::skip]
macro_rules! finish_msg {
	($($args:tt)*) => {
		println!("{}", $crate::msg::Paint::default(format_args!($($args)*)).bold())
	}
}
