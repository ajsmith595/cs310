# Known Issues
## Server
To run with debugging (mostly to find segfaults), run with GDB using the command `cargo with gdb -- run`
### Segfaults
- "typefind:sink" received signal SIGABRT, Aborted
    - Only found this by pretty much spamming changes in the UI; guessing perhaps two GStreamer processes are open at once, or something like that?
- "typefind:sink" received signal SIGSEGV, Segmentation fault.

- these seem like race conditions. Trying with GST_DEBUG=4 outputs all debug messages, but the error does not occur - I'm thinking this is because the messages are making things too slow to happen. I'm thinking as one GST process ends, the other one starts, but I'm guessing there's a "collision" period whereby both are running, and I'm guessing something's messing up as a result



179>, which means that the other core child 'audiourisource627' of the same type can not be added to the track. Consider connecting to GESTimeline::select-tracks-for-objects to be able to specify which core element should land in the track
0:05:05.796405195  1697 0x7fb15009a300 ERROR              ges-asset ges-asset.c:1290:ges_asset_request: Failed to reload the asset for id file:////home/ajsmith/cs310/server/application_data/source/ecfc3f93-3840-4a44-8972-4784b8444ceb
0:05:05.796610074  1697 0x7fb15009a300 WARN              discoverer gstdiscoverer.c:2024:start_discovering: No URI to process
0:05:05.796886404  1697 0x7fb15009a300 WARN                 basesrc gstbasesrc.c:3688:gst_base_src_start_complete:<source> pad not activated yet
0:05:05.797217073  1697 0x7fb15009a300 WARN                 basesrc gstbasesrc.c:3688:gst_base_src_start_complete:<source> pad not activated yet
0:05:05.799019902  1697 0x7fb05c055a70 WARN                     ges ges-timeline-element.c:1163:ges_timeline_element_set_inpoint:<GESUriClip@0x7fb18004e8b0> Can not set an in-point of 99:99:99.999999999 because it exceeds the element's max-duration: 0:00:31.734000000
0:05:05.799103757  1697 0x7fb13400e360 WARN                 qtdemux qtdemux_types.c:244:qtdemux_type_get: unknown QuickTime node type sgpd
0:05:05.799135007  1697 0x7fb13400e360 WARN                 qtdemux qtdemux_types.c:244:qtdemux_type_get: unknown QuickTime node type sbgp
0:05:05.799162117  1697 0x7fb13400e360 WARN                 qtdemux qtdemux_types.c:244:qtdemux_type_get: unknown QuickTime node type sgpd
0:05:05.799203917  1697 0x7fb13400e360 WARN                 qtdemux qtdemux_types.c:244:qtdemux_type_get: unknown QuickTime node type sbgp
0:05:05.799164972  1697 0x7fb05c055a70 ERROR              ges-asset ges-asset.c:1290:ges_asset_request: Failed to reload the asset for id file:////home/ajsmith/cs310/server/application_data/source/ecfc3f93-3840-4a44-8972-4784b8444ceb
0:05:05.799259547  1697 0x7fb13400e360 WARN                 qtdemux qtdemux.c:3066:qtdemux_parse_trex:<qtdemux85> failed to find fragment defaults for stream 1
**
GES:ERROR:../ges/ges-video-uri-source.c:69:ges_video_uri_source_needs_converters: assertion failed: (asset)
0:05:05.799352216  1697 0x7fb13400e360 WARN                 qtdemux qtdemux.c:3066:qtdemux_parse_trex:<qtdemux85> failed to find fragment defaults for stream 2
Bail out! GES:ERROR:../ges/ges-video-uri-source.c:69:ges_video_uri_source_needs_converters: assertion failed: (asset)
0:05:05.799405696  1697 0x7fb13400e360 WARN                 qtdemux qtdemux.c:3066:qtdemux_parse_trex:<qtdemux85> failed to find fragment defaults for stream 3
Aborted