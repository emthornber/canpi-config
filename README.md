# canpi-config
Rust library to handle configuration files for CANPiCap and CANPiZero

This crate provides functionality to read and write the canpi server configuration
and to define which configuration items can be changed or viewed by the user and which are hidden.

There is a JSON file that defines the configuration item format and default values.
This file is validated against a JSON schema generated from internal structures and then loaded to those structures.
The canpi INI file, if it exists, is read to update current value of the configuration items so the ConfigHash
becomes the single source of truth.

There is the means to export the current values as an INI file.
