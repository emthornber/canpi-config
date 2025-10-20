# Change Log

canpi-config (0.1.9) bookworm; urgency=low

- The 'format' field in the Attributes structure now has a data type of an
Either enum.  
The Left entry is a String intended to hold a Regexp to validate
the user input.  
The Right entry is a Vector of Strings to be used in a Drop Down
List (HTML <select> tag).

-- Mark Thornber <mark.thornber@gmail.com> Mon, 20 Oct 2025 14:13:25 +0100

canpi-config (0.1.8) bookworm; urgency=low

- Improved Pkg test suite

-- Mark Thornber <mark.thornber@gmail.com> Fri, 05 Sep 2025 10:37:29 +0100

canpi-config (0.1.7) bookworm; urgency=low

- Added optional service_name field to Package structure

-- Mark Thornber <mark.thornber@gmail.com> Sat, 09 Aug 2025 12:49:18 +0100

canpi-config (0.1.6) bookworm; urgency=low

- Updated dependencies
- Reworked JSON schema validation code
- Updated Rust edition to 2021

-- Mark Thornber <mark.thornber@gmail.com> Fri, 29 Jul 2025 00:10:18 +0100

canpi-config (0.1.5) bookworm; urgency=low

- Issue 1 refactoring

-- Mark Thornber <mark.thornber@gmail.com> Fri, 3 Jun 2025 00:08:58 +0100

canpi-config (0.1.4) bookworm; urgency=low

- Added this file and INSTALL.md
- Updated Cargo.toml dependency versions

-- Mark Thornber <mark.thornber@gmail.com> Fri, 28 Feb 2025 08:56:20 +0000
