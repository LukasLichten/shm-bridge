# Shared Memory Bridge &emsp; [![Build Status]][actions] [![License: MIT]][license]

[License: MIT]: https://img.shields.io/badge/License-MIT-yellow.svg
[license]: https://opensource.org/licenses/MIT
[Build Status]: https://img.shields.io/github/actions/workflow/status/poljar/shm-bridge/ci.yml?branch=main
[actions]: https://github.com/poljar/shm-bridge/actions/workflows/ci.yml


Share memory between a Windows application running under Wine/Proton and Linux.

This allows you to expose [named shared memory] a Windows application uses under
Linux. The Linux application can then [`mmap(2)`] the named shared memory file and
read its data.

## History

This is a Rust port of [`oculus-wine-wrapper`]. The main difference here is
that [`oculus-wine-wrapper`] exposed an existing `/dev/shm` backed file, that
the `oculusd` daemon creates, as [named shared memory] using Wine.

We on the other hand go the other direction. We want to re-expose named shared
memory a Windows executable creates under Linux. For this to work, we first
create a file on a [`tmpfs`] file system, then we create the named shared memory
mapping, backed by the [`tmpfs`] file. We need to create the named shared memory
before the Windows executable does, then it will just reuse the, now [`tmpfs`]
backed named shared memory as if it created it itself.

```mermaid
flowchart LR
    windows[Windows Application]
    linux[Linux Application]

    subgraph bridge[Shared Memory Bridge]
        direction TB
        file[tmpfs File]
        shared_memory[Named Shared Memory]
        
        file <==> shared_memory
    end

    linux <==> bridge
    bridge <==> windows
```

Once the Windows application uses the named shared memory we created, the data
the Windows application exposes using the named shared memory will be found in
the [`tmpfs`] file as well. Linux applications then can just open and [`mmap(2)`]
the [`tmpfs`] file to read shared memory.

## Installation 

The installation of `shm-bridge` requires [Rust], we're going to assume that you
have already [installed Rust][rust-install]. Since this is a Windows application
that is meant to be run under Wine/Proton you'll have to add a Windows target to
your Rust installation.

```bash
$ rustup target add x86_64-pc-windows-gnu
```

After the target has been installed, `shm-bridge` can be installed using
[`cargo`]. From the root directory of the project, launch:

```bash
$ cargo install
```

## Usage

The bridge requires to be run under Wine or Proton, it's recommended to install
[`protontricks`] for ease of use.


The bridge should be launched inside the container of the application:

```bash
$ protontricks-launch --appid APPID shm-bridge.exe --map [map_name] --size [map_size]
```
You can pass in multiple maps, but you need to pass as many `--map` as `--size`

### Finding the application ID

To find out the `APPID` you can use `protontricks` itself:

```bash
$ protontricks -s "Assetto Corsa"

Found the following games:
Assetto Corsa Competizione (805550)

To run Protontricks for the chosen game, run:
$ protontricks APPID COMMAND

NOTE: A game must be launched at least once before Protontricks can find the game.
```

### Launching the bridge

Now you can launch the bridge in the container of the game:

```bash
$ protontricks-launch --appid 805550 shm-bridge.exe -m acpmf_crewchief acpmf_static acpmf_physics acpmf_graphics -s 15660 2048 2048 2048

Found a tmpfs filesystem at /dev/shm/
Created a tmpfs backed mapping for acpmf_crewchief with size 15660
Created a tmpfs backed mapping for acpmf_static with size 2048
Created a tmpfs backed mapping for acpmf_physics with size 2048
Created a tmpfs backed mapping for acpmf_graphics with size 2048
All mappings were successfully created, press CTRL-C to exit.

```

### Known Issues with Protontricks
The flatpak version does not work.  
Even if you give permission to access the exe (or move it for example into the prefix),
there is a weird bug relating the fsync, that when the game has been launched you fsync won't launch on this program and you get an fsync error on attempting to mount the memory map.  
  
The other bug is that you can't launch the game if this software is running in the prefix already, the game will get stuck on launching till we exit.  
However, once the game jumps to Running state we can launch fine, and as most games mount the memorymaps only when going into a Session, so we have ample time.  

### Cleaning up
SigKill and sometimes Ctrl-C actions are consumed by protontricks and terminate our bridge instead of letting it terminate properly.
This means files are left behind in `dev/shm`.
This is not a big deal, these files will be overwritten be `shm_bridge` on restart,
but if you want to clean after improper shutdown use `--clean-up`
(you will still need to pass the names of the maps)

## Supported titles

The Shared Memory Bridge (should) support any title using memory maps,
but you need to look up the name and size of memory maps for these titles.  

Above launch command works for:
* [Assetto Corsa][ac]
* [Assetto Corsa Competizione][acc]

### Assetto Corsa / Assetto Corsa Competizione

To access the shared memory you can [`shm_open(3)`] and [`mmap(2)`] the files
listed in the output of `shm-bridge.exe`.

For example, if we run the bridge we will see an output like:

```
...
Found a tmpfs filesystem at /dev/shm/
Created a tmpfs backed mapping for acpmf_static with size 2048
...
```

This tells us that we can open a file under `/dev/shm/acpmf_static` and map the
memory into our process. To interpret the bytes in the file, you'll have to use
structures from the games SDK. The [simapi] project contains definitions of such
structures, alternatively the Rust library [simetry] contains them as well.

To quickly test if the bridge correctly works you can also use some applications
that use [simapi], for example [gilles]. Another option would be to use one of
the examples in the [simetry] repository:

```bash
$ cargo run --example assetto_corsa_competizione_get_data

```

**Warning**: This example currently requires the usage of a fork of [simetry],
which can be found at: https://github.com/poljar/simetry/

## Development

To modify and develop `shm-bridge` you'll have to install [Rust] and the Windows
Rust target, please take a look at the [installation](#installation) section.

Cargo has been set up to use the `x86_64-pc-windows-gnu` target by default. This
means that, once the correct target has been installed, `cargo
check`, `build`, or `install` work as expected. Wine has been set up as the
default runner, which makes `cargo run` work.  
Although `cargo run` is not recommended, is it would run in the default wine prefix instead of the game's protonprefix, you can use `make` for this end goal.

```bash
$ make help
Builds and test the shm-bridge
make:          Builds
make release:  Builds in release mode
make ac:       Build and Run Memory Maps in the AC prefix
make acc:      Build and Run Memory Maps in the ACC prefix
make rf2:      Build and Run Memory Maps in the rF2 prefix
make clean-up: Removes Stale Memory Maps
make clear:    Clean-up but also runs Cargo Clean
make help:     This Printout
```

## Similar/Related Projects

A couple of similar projects exists in various languages, they all seem to
require multiple binaries, one Linux binary to create the `/dev/shm` backed files
and one Windows binary to create the named shared memory utilizing the file.

* [simshmbridge] - Wrapper programs to map shared memory from Linux for access
                   by Wine and Proton.
* [wineshm-go] - This package retrieves a Wine shared memory map file descriptor
                 and makes it available in Linux.
* [wine-linux-shm-adapter] - Wrapper programs to map shared memory from a Wine
                             process into a Linux process.

## License

Licensed under [The MIT][license] License.

## Copyright

Copyright © 2024, [Damir Jelić](mailto:poljar@termina.org.uk).

[`protontricks`]: https://github.com/Matoking/protontricks/
[license]: https://github.com/poljar/shm-bridge/blob/main/LICENSE
[named shared memory]: https://learn.microsoft.com/en-us/windows/win32/memory/creating-named-shared-memory
[`oculus-wine-wrapper`]: https://github.com/feilen/oculus-wine-wrapper/
[ac]: https://store.steampowered.com/app/805550/Assetto_Corsa_Competizione/
[acc]: https://store.steampowered.com/app/805550/Assetto_Corsa_Competizione/
[`tmpfs`]: https://www.kernel.org/doc/html/latest/filesystems/tmpfs.html
[`mmap(2)`]: https://man7.org/linux/man-pages/man2/mmap.2.html
[simshmbridge]: https://github.com/spacefreak18/simshmbridge
[wineshm-go]: https://github.com/LeonB/wineshm-go
[wine-linux-shm-adapter]: https://github.com/Spacefreak18/wine-linux-shm-adapter
[rust-install]: https://www.rust-lang.org/tools/install
[Rust]: https://www.rust-lang.org/
[`cargo`]: https://doc.rust-lang.org/cargo/
[`shm_open(3)`]: https://man7.org/linux/man-pages/man3/shm_open.3.html
[simapi]: https://github.com/spacefreak18/simapi
[simetry]: https://github.com/adnanademovic/simetry/
[gilles]: https://github.com/Spacefreak18/gilles
