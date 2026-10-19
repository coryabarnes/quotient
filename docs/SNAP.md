# Snap distribution

Quotient's Snap is built from source on Ubuntu 24.04 using the `core24` base
and Snapcraft's GNOME desktop extension. The initial target is x86-64 (`amd64`).
The source recipe is `snap/snapcraft.yaml`; the build and smoke checks are in
`.github/workflows/snap.yml`.

## Current release

Version 0.1.0, revision 1, is published on `latest/edge`. The Snap passed
Canonical's automated review. Its build, installation, representative calculations
and desktop startup passed on Ubuntu 24.04; calculations also passed on Fedora 44.

```sh
sudo snap install quotient --edge
```

The first release is a testing build. A stable-channel release follows the
remaining desktop acceptance checks below.

## Build and test

With Snapcraft and a supported build provider installed:

```sh
snapcraft
sudo snap install --dangerous ./quotient_0.1.0_amd64.snap
snap run quotient -c "5 km to mi"
snap run quotient
```

Use `snap run quotient` explicitly if a native copy is also installed. Both the
native app and the Snap may appear in your launcher while testing.

GitHub Actions builds the package, installs it in a clean Ubuntu runner, checks
representative calculations and verifies that the desktop process stays open.
Download the `quotient-snap-amd64` artifact from a successful build. These checks
do not replace testing file dialogs, clipboard access, saving, and graphics on
your own desktop.

The Snap uses strict confinement and requests access to non-hidden files in
the user's home directory. Desktop and graphics interfaces come from the GNOME
extension. It does not request network access. Its saved preferences and
untitled document are separate from a native installation's data. Confinement
support depends on the host; Fedora currently reports partial support.

## Publish an initial testing release

1. Register the package name in the [Snapcraft developer dashboard](https://dashboard.snapcraft.io/register-snap/).
   A missing public listing does not guarantee that a name can be registered.
2. Sign in locally with `snapcraft login`. Enter account credentials only into
   Snapcraft's authentication prompt, never into repository files or an issue.
3. Upload the tested package to the edge channel:

   ```sh
   snapcraft upload --release=edge ./quotient_0.1.0_amd64.snap
   ```

4. Check the store review and install the published revision:

   ```sh
   snapcraft status quotient
   sudo snap install quotient --edge
   ```

5. Complete the listing with the bundled icon and the screenshots in
   `docs/screenshots/`. The package description comes from `snap/snapcraft.yaml`.

The recipe currently uses `grade: devel` so it is intended for testing channels.
Before a stable release, finish the desktop checks, change the grade to `stable`,
rebuild, test the new revision and publish it. Store acceptance is determined by
Canonical's review process. This is independent of Fedora's repositories.

## Desktop acceptance checks

- Launch from the application menu; confirm launcher and taskbar icons.
- Calculate units, percentages, complex values and high-precision expressions.
- Resize the window; confirm long and multiline results stay readable.
- Open and save a file through the file dialog, then reopen the saved file.
- Click a result and paste it into another application.
- Close and reopen; verify preferences and untitled work persist.
- Check both light and dark themes.

Screenshots in this repository are rendered from Quotient's own GUI test harness,
using its welcome document and built-in help. They are not images of a tested
Snap installation.
