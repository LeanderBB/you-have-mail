# You Have Mail Common

Shared code for the [You Have Mail Android](https://github.com/LeanderBB/you-have-mail) 
and the [You Have Mail CLI](https://github.com/LeanderBB/you-have-mail-cli) applications.

Get notified when you receive an email in your email account. This may be useful for cases where you only wish to be 
notified when your email account has a new message and the default notification mechanism does not work 
(e.g: Android without Google Play Services) or do not wish to have the web interface/email client open at all times.



## Supported Backends

The application structure has been made backend agnostics, so it should be possible to add different providers in the
future. Currently, the following email providers are supported:

* [Proton Mail](https://mail.proton.me) - This backend only reports new messages in the INBOX mailbox