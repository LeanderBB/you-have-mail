# You Have Mail Android

Here you can find the android application that uses the You Have Mail shared service code.

## Architecture

This app has been specifically designed to work in cases where Google Play services are not available. The App launches
a Foreground Service that polls the accounts every 15 seconds. Once an account reports a new message has arrived it
will create a notification.

When the App knows about which backend maps to which Android application, it will try to launch that app if the user
clicks on the application.

## Compatibility

The Application is only available for x86_64 and Aarch64 and is has API 29 as minimum requirements. It has been tested
on a Pixel 3a running the last compatible version of [Graphene OS](https://grapheneos.org/).

## Security

Session tokens are stored using the `EncryptedSharedPreferences` API.

## Building and Installing using Android Studio

This application is built in Android Studio. It uses Rust programming language.  
In order to build and run this application in Android Studio install Rust in your environment.  
Below commands are for Linux Debian/Ubuntu:  
```
sudo apt-get install -y python-is-python3 golang-go;  
# Install Rust using official method  
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh  
rustup default stable;  
rustup target add armv7-linux-androideabi;  
rustup target add aarch64-linux-android;  
rustup target add x86_64-linux-android;  
```
Alternatively use suitable docker image with Rust preinstalled instead of manual installation above.

