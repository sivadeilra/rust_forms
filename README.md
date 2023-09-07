# Rust Forms for Windows

This crate allows you to build simple GUI apps on Windows, using Rust.

# Why only Windows?

This crate does not directly implement the GUI controls and logic. All it does is expose the
GUI controls of the host platform, which in this case is Windows.

There are many other projects which aim to provide cross-platform GUIs for Rust.  See
[Are We GUI Yet?](https://areweguiyet.com/) for a comprehensive list of GUI projects for Rust.

# Goals

Rust Forms aims to be:

* Fast to build. It has a very small dependency set.

* Small, self-contained binary output. There is no separate runtime, such as the Electron executable.

* Reasonably useful, familiar GUI. Since this just uses the native controls of Windows, the apps
  look and feel like Windows apps.

* Interop with many existing Windows GUI APIs. With some hackery and use of the `windows-rs` crate,
  you can build apps that use `rust_forms` 

Rust Forms _does not_ aim for these goals:

* No attractive UI. If you're looking for beautiful, flexible UIs with all the power of HTML/CSS,
  then this is not the toolkit to use.

* No attractive API.  The API is intended to be useful, but it has been necessary to make tradeoffs
  in order to work within the constraints of the traditional message-loop GUI environment of Windows.

* Not cross-platform.  Rust Forms only runs on Windows, because it relies entirely on native
  controls. Its APIs are driven by the needs of the native controls that it uses.

# Why a new GUI toolkit?

The motivation for Rust Forms is to provide a _fast_, _simple_ API for throwing together useful
tools. It is not meant to provide a comprehensive GUI, or even an attractive GUI.  In my work, I
often need to create a tool for visualizing some data or exploring some complex data set.

In the past, WinForms was one of my favorite toolkits because it was easy to use, efficient, and
reasonably rich. Now that most of my development has moved to Rust, I need something that fills
the same niche. For the work that I do on Windows, cross-platform is not relevant.

Another motivation for this, is that the Windows GUI controls (such as List View) actually do a
reasonable job at handling moderately-large data sets.  The same is not true for many of the
immediate-mode APIs that are still under development; they require rebuilding the entire visual
tree every time the app state changes.  For list views that contain 50,000 items, that's just not
scalabe.

# License

Rust Forms is dual-licensed under the MIT and Apache-2.0 licenses.

# Contributions

Contributions are welcome.  Contributions that advance the goals, without dragging this into a
whole new design space (such as cross-platform), are welcome.

# Author

* Arlie Davis, `sivadeilra` on GitHub
