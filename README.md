# Raspberry Pi Weather Station Software

Handles the hardware connections (via the RPi's GPIO), data collection, locally-hosted server, and data upload.

All of this is programmed and tested wirelessly through a Raspberry Pi (thank God and Microsoft for VSCode).

TODO:
 - Set up and refine data collection.
 - Create a local database (thinking PostgreSQL on the Pi) and store data collected as it comes.
 - Set up a new server in the cloud strictly for weather data, likely in Rust (cause why not).
 - Set up the Pi to ping the server as well as auto setup and data sendoff.
 - Set up data to be sent to Weather Underground.
