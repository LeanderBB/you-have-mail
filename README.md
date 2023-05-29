# You Have Mail

Small application to notify you when you receive an email in your email account. This may be useful for cases where
you only wish to be notified when your email account has a new message and the default notification mechanism 
does not work (e.g: Android without Google Play Services) or do not wish to have the web interface/email client open at
all times.

## Supported Backends

The application structure has been made backend agnostics, so it should be possible to add different providers in the 
future. Currently, the following email providers are supported:

* [Proton Mail](https://mail.proton.me) - This backend only reports new messages in the INBOX and **Single Password mode** only at the moment.

## Structure 

This repository is split into the following projects:

* [you-have-mail-common](you-have-mail-common): Shared code for the project
* [you-have-mail-mobile](you-have-mail-mobile): Shared code for mobile bindings
* [you-have-mail-android](you-have-mail-android): Android Application


## Donations

* Monero: `86CBWfyMFAYM6a7zJUmhj5Xp7hmm8LkVRE9xSHuJ28Lti22KGxGXSNBUGkJBw7PvJC5RWJfEvqkJJjhsaJPT8LYB4kbXc2S`
