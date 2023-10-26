# You Have Mail

Small application to notify you when you receive an email in your email account. This may be useful for cases where
you only wish to be notified when your email account has a new message and the default notification mechanism
does not work (e.g: Android without Google Play Services) or do not wish to have the web interface/email client open at
all times.

If you want these features in CLI package, please see [You Have Mail CLI](https://github.com/LeanderBB/you-have-mail-cli).

## Supported Backends

The application structure has been made backend agnostics, so it should be possible to add different providers in the
future. Currently, the following email providers are supported:

* [Proton Mail](https://mail.proton.me) - This backend only reports new messages in the INBOX mailbox

## Structure

This repository is split into the following projects:

* [you-have-mail-mobile](you-have-mail-mobile): Shared code for mobile bindings
* [you-have-mail-android](you-have-mail-android): Android Application

## Download

Please only download the latest stable releases from:

* Github: [Releases](https://github.com/LeanderBB/you-have-mail/releases)
* F-Droid: [Link](https://f-droid.org/packages/dev.lbeernaert.youhavemail/)

[<img src="https://f-droid.org/badge/get-it-on.png" alt="Get it on F-Droid" height="60">](https://f-droid.org/packages/dev.lbeernaert.youhavemail/)


## Donations

If you wish to donate to this project, consider donating to the
[GrapheneOS](https://grapheneos.org/donate) project instead :).
