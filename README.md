# canpi-config
Rust library to handle configuration files for CANPiCap and CANPiZero

This crate provides functionality to read and write the canpi server configuration
and to define which configuration items can be changed or viewed by the user and which are hidden.

There is a JSON file that defines the configuration item format and default values
along with a matching schema file.  This file is loaded to the ConfigHash.  The canpi INI file,
if it exists, is read to update current value of the configuration items so the ConfigHash
becomes the single source of truth.

There is a function to write the ConfigHash as an INI file.
