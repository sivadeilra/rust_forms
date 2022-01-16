

Let's say we want to do better than just WinForms.
We want to have the UI thread be separate from the main app thread.
Communication between UI thread and main thread is always asynchronous.
UI thread can always:
* render
* recompute layout
* respond to focus changes
* allow user to scroll
* allow user to edit text boxes

When the user does something that changes the state of the app in some
significant way, then the UI thread sends a message to a worker thread, and
then blocks further user input. Rendering still happens, though. When the
app thread responds, it unblocks the UI thread.


The first thing an app does is to create a UI thread. The app gets back a
handle to that UI thread, which allows the app thread to control the UI thread
(send it messages).



# Goals

* Simplicity: Make it easy to write simple, useful apps.
    + Make it easy to compile and distribute. No redistributable / shared DLLs.
    + App startup should be extremely simple. Create a Window, add some
      controls to it, enter event loop.

* Single-threaded: This library makes no attempt to provide a multi-threaded
  or asynchronous model. The app can do that, and we'll use `Send` and `Sync`
  to provide some guard rails.

  Most of the functions will take shared references, not mutable references,
  even for operations that have mutable semantics, such as setting the layout
  preferences for a control. Some objects, such as ListView, might have a way
  to lock the object (and guard against re-entrancy), for working with its
  contents / items.

* A fixed set of controls. No attempt is made to make this a general-purpose
  toolkit for writing arbitrary, complex controls. _Maybe_ at some point we
  provide a way to hook "paint" requests and such, but that is not a v1 goal.

* A fixed set of layout policies for controls. We will provide a set of layouts
  that meet the needs of the most common UI types, and beyond that, we will
  require that apps use absolute positioning and sizing.

  Layout policies:

  * Fixed
  * Flex grid

* Even handlers will always run on the main thread, not any kind of background
  or worker thread.


# How to create controls

Creating a control is simple: call `Foo::new()`. Then call methods on it that
set its layout properties, visual attributes, etc. Then place it in a layout.

Each `Window` provides layout services for its contained controls. As much as
possible, we will provide an API that looks like WinForms, with a very simple
means to wire up controls to a form. Each `Window` has a `Controls` collection,
with methods to add controls. Each `Control` has a nested `Controls` collection,
for the same thing.

* Each `Control` has a way to set its layout properties. For example:

  ```rust
  fn set_layout(&self, layout: Layout);
  fn set_min_size(&self, min_size: Option<Size>);
  fn set_max_size(&self, max_size: Option<Size>);
  fn set_grid_layout(&self, row: i32, col: i32);
  fn set_grid_alignment(&self, horizontal: HorizontalAlignment, vertical: VerticalAlignment);


  enum Layout {
      Fixed(Size, Point),
      Grid { row: i32, col: i32 },
      Docked {
  }



  ```


