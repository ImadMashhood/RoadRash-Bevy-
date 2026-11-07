# Actual downloaded RC6 standalone package

Passed using the published Ubuntu archive `08d7de03…`, game `a26b1530…`, frozen source `d9dac514…`. All 712 manifest files and the 369 dependency notices plus bundled font license were independently verified; commercial payload is absent.

The external read-only application ran in Bubblewrap with the checkout hidden and Cargo absent from PATH. Missing-disc guidance appeared correctly. A fresh owned-disc ISO imported and verified all 341 files and converted the catalog, then cached launch reopened successfully.

Replacing the previously verified actual RC5 application with actual RC6 preserved all three profile JSON files, the active-import pointer and all 6,486 imported/catalog files. Removing only the disposable application preserved those same user files and the original ISO. Production startup reopened the preserved ordinary keyboard profile. These headless setup/reopen checks are separate from the actual ordinary keyboard recovery walkthrough.

Private logs and the all-file preservation digest remain under `local/rc6-package-download-sandbox`. No commercial files or captures are published. No performance or owner G5 claim is made.
