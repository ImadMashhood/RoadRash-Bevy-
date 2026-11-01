# Native-art Linux candidate

Extract the application archive into a directory you own and run `./RoadRash`.
The graphical launcher asks for your own supported Road Rash ISO or ZIP, verifies
the disc, imports its files and prepares the original graphics. This can take a
few minutes. Python 3 and Zenity are required for the launcher; the game needs
Vulkan-capable Linux graphics, X11/XWayland and ALSA/PipeWire. Install FFmpeg and
ffprobe for films. No game disc, converted artwork, soundtrack or film ships in
the application. Unsupported disc editions fail verification rather than being
silently accepted. The recorded acceptance platform is Fedora Linux x86_64.

The menus use Up/Down and Enter/Escape, or controller D-pad and South/East.
Selection is marked with `>` and text, independently of color. Long lists scroll
with focus; dialogue, credits, specifications and standings have page controls.
In the ISO picker, Ctrl+L opens its location field; enter your file path and press
Enter. Import progress can be canceled, retaining the previous verified import.

Choose Thrash for a quick race: select a rider, course and optional level. Choose
New Big Game for a career: select a rider, then Race on the street. Finish in the
first three to qualify on a course. Qualify on all five courses to advance; finish
level five to reach the finale. Replaying a qualified course earns normal prizes
without duplicating qualifications. The showroom buys bikes with the current
bike's half-price trade-in; repair prices are shown before confirmation. Cash,
qualifications, bike condition and settlement IDs persist in a career slot.

Default keyboard race controls are Up/W throttle, Down/S brake, Left/A and
Right/D steer, Z punch, X kick, C backhand, Q/E independent lean, Left Shift
nitro and V intentional stationary dismount. Arrows move the rider on foot
toward the fallen bike. Nitro requires a compatible bike and available charges;
throttle release, braking, depletion or a crash cancels the active boost. V is
edge-triggered and allowed only near standstill. Escape pauses. Release driving
controls before resuming; the neutral gate prevents an unexpected held throttle.
Check Configure Input for the active remappable keyboard bindings.

Controller defaults: right/left analog triggers accelerate/brake, left stick
steers, West punches, North kicks, left bumper backhands, right bumper boosts,
Select dismounts, D-pad Up/Down leans, and Start pauses/resumes. Newly connected
held controls are quarantined until released. Menu navigation owns its controls
while menus are open. Physical controller models beyond the recorded Linux
virtual/backend checks have not been certified.

Restroom → Settings controls language (English, French, Spanish, German or
Italian), text scale (75–150%), high contrast, reduced camera motion, reduced
flashes, sound captions, audio buses, automatic videos and vsync. Modern keyed
labels and original local dialogue/credits use the selected language; proper
names and some diagnostic/result prose retain English. Source menu artwork is
approximated by accessible panels. Sound captions describe gameplay cues; they
are **not film speech transcripts**. Reduced flashes suppress automatic films;
explicit film playback remains available. Films: Space pauses, Enter/Escape or
controller South/East skips and returns to the owning screen. The jukebox has
the fourteen supplied PCM soundtrack tracks. Legacy MIDI/SBK synthesis is not
implemented; selecting the supplied PCM soundtrack is the native release policy.

Career slots, explicit backup recovery and preferences are under Restroom. A
corrupt primary save is reported; recovery is an explicit action, never a silent
cash/progression rollback. Original Windows binary saves cannot be imported.
Settings and bindings are independent of career slots and survive graphics reset.

Application files can be read-only. Imported data defaults to
`$XDG_DATA_HOME/roadrash-rust` (normally `~/.local/share/roadrash-rust`), while
preferences use the platform configuration directory. The game supports
`--profile-root PATH` for an isolated profile. The launcher supports
`--data-root PATH` and `--source PATH`; `--headless --prepare-only` verifies and
prepares a supplied disc without opening the game or a file picker.

Upgrade by closing the game and replacing the application directory. Keep the
user data/config directories. Uninstall by deleting the application directory;
this intentionally retains saves and separately imported data. Delete those user
directories only if you also want to erase your profile/import. Never delete the
original ISO as part of application uninstall.

If launch fails, read `launcher.log` in the launcher's data directory; the previous
rotated log is `launcher.previous.log`. Missing disc: choose a supported ISO or ZIP.
Missing movie decoder: install FFmpeg/ffprobe or disable automatic videos. Missing
audio device: gameplay continues silently and retries the device. Graphics startup
failure: check the Vulkan driver and X11/XWayland session. Damaged converted cache:
close the application, move `catalog-rr-content-3` aside and relaunch to rebuild it.
Keep `import` and career/config directories. Do not attach the disc, import,
converted catalog, raw replay setup, film/audio, dialogue dump or screenshots to
public bug reports. Start with the error text, OS/driver version and application
checksum; redact personal paths in logs before sharing.

For review, follow: first-launch import → Thrash → race → pause → resume → quit
race → results → Main → Big Game → save slot → close/reopen → resume → Settings →
change text scale/language → close/reopen → verify persistence → Exit. A completed
automated native career is separate evidence from a human review of game feel.

This is an offline recreation candidate with original textures. Network play,
AI-upscaled production assets, a 3D renderer, original-runtime parity and other
operating systems are outside this candidate's acceptance. Enhancement production
stays locked until the native release gate has an explicit recorded signoff.
