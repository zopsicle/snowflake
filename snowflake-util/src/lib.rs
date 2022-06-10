//! Snowflake-specific utility items.
//!
//! The utilities in this crate are not general-purpose;
//! they have quirks specific to the Snowflake build system.
//! It is advised to use these only when extending or embedding Snowflake.

#![doc(html_logo_url = "/snowflake-manual/_static/logo.svg")]
#![feature(concat_bytes)]
#![feature(io_safety)]
#![warn(missing_docs)]

/// Markdown text telling the reader to refer to
/// the manual for definitions of unfamiliar terms.
#[macro_export]
macro_rules! see_manual
{
    () => {
        concat!(
            "The documentation for this crate does not define all terms.\n",
            "Refer to the [Snowflake manual] for a thorough description\n",
            "of all the terms and their concepts involved.\n",
            "Especially the [index] might be of interest.\n",
            "\n",
            "[Snowflake manual]: /snowflake-manual/index.html\n",
            "[index]: /snowflake-manual/genindex.html\n",
        )
    };
}

pub mod basename;
pub mod hash;
