# canpi-config
Rust library to handle configuration files for CANPiCap and CANPiZero

This crate provides functionality to read and write the canpi server configuration files
and to define which configuration items can be changed or viewed by the user and which are hidden.

There is a JSON file that defines the configuration item format and default values.
This file is validated against a JSON schema generated from internal structures and then loaded internally.
A canpi INI file is read to determine which of the configuration items are being used in this instance.
The current values from the INI file are then merged with the selected item definitions to create an instance
of a ConfigHash.

There is the means to export the current values as an INI file.
