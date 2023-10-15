# Cipherpad

## Project Description

Cipherpad is a note editor uniquely designed with user privacy at its forefront.
Within Cipherpad, data is organized in a structured manner, with each individual item — be it a note, an image, an audio clip, or other files — encrypted at its own level.
These individual items are referred to as "Pads" within Cipherpad. Importantly, a single password is utilized for the encryption of all the Pads.

### Core Features:

- **Unified Password Encryption**: A singular password ensures the encryption of all Pads, providing both convenience and a layer of security.
  
- **Pad-Level Encryption**: Every piece of content, categorized as a "Pad", undergoes individual encryption. This approach guarantees the confidentiality of a variety of data types, from text to images to audio.
  
- **Versatile Content Storage**: Cipherpad is not confined to textual notes. Users can securely store an array of file types as Pads.

- **Markdown Support**: Text within Cipherpad can be authored in Markdown format, offering a rich-text experience as the content is rendered within the application.
  
- **Cross-Platform Experience**: With Tauri as its backbone, Cipherpad promises a uniform and user-friendly experience across different desktop platforms.

At its core, Cipherpad is dedicated to offering a comprehensive and secure note-taking environment, enabling users to document their thoughts, ideas, and memories with assured privacy.

## Technical Details

### Encryption

Upon initializing a new Cipherpad using a password, a master key is generated leveraging the `argon2` algorithm.
For every new Pad being saved, its data undergoes encryption via the AES-256-GCM method, utilizing an HKDF key derived from the aforementioned master key.
The encryption process is grounded on the `argon2` and `ring` crates to ensure robust security.

### File Format

Internally, a Cipherpad file manifests as an SQLite database that comprises a singular `node` table.
Embracing a hierarchical design, Pads can nest other Pads within them, drawing parallels to a file-system's tree structure.
While the overall architecture of this structure is visible, Cipherpad's primary emphasis rests on ensuring rigorous encryption at the individual Pad level.

## Building

To build the Cipherpad application, first follow Tauri's [Prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites) guide.

Install Cipherpad's node dependencies then run Tauri's build script.

```
$ npm install
$ npm run tauri build
```

A Dockerfile is provided to build a redistrubtable for Linux environments. It's based off of Ubuntu 20.04 to provide a low GLIBC version requirement.
A workaround for [this](https://github.com/tauri-apps/tauri/issues/1355) Tauri issue.