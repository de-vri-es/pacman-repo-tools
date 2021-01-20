#[doc(hidden)]
pub use yansi::Paint;

#[doc(hidden)]
pub fn use_color() -> bool {
	let force_off = std::env::var_os("CLICOLOR").map(|x| x == "0").unwrap_or(false);
	let force_on = std::env::var_os("CLICOLOR_FORCE").map(|x| x == "1").unwrap_or(false);
	let is_tty = atty::is(atty::Stream::Stdout);
	!force_off && (force_on || is_tty)
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
