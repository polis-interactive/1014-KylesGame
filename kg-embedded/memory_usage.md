## Current size

Binary: 852 kB

INFO  free=153600
└─ kg_embedded::__core1_task_task::{async_fn#0} @ src/main.rs:211 
INFO  Runnng thumbstick
└─ kg_embedded::thumbstick::__thumbstick_task_task::{async_fn#0} @ src/thumbstick.rs:62  
INFO  free=56076
└─ kg_embedded::__core0_task_task::{async_fn#0}::{closure#0} @ src/main.rs:182 
INFO  free=49116
└─ kg_embedded::__core0_task_task::{async_fn#0}::{closure#1} @ src/main.rs:185 

So we are using quite a lot of the heap, but still have a 3rd free... I'm wondering why its so picky

## Removing bevy_rand in favor of small rng

Binary: 804 kB

shaved off a bit, but I still think I could do without SmallRng and just use the entropy source directly

INFO  free=153600
└─ kg_embedded::__core1_task_task::{async_fn#0} @ src/main.rs:211 
INFO  Runnng thumbstick
└─ kg_embedded::thumbstick::__thumbstick_task_task::{async_fn#0} @ src/thumbstick.rs:62  
INFO  free=61524
└─ kg_embedded::__core0_task_task::{async_fn#0}::{closure#0} @ src/main.rs:182 
INFO  free=54260
└─ kg_embedded::__core0_task_task::{async_fn#0}::{closure#1} @ src/main.rs:185 

Saved a little bit of heap, but just like 5kB, meh

I was also able to remove getrandom usage, but that didn't save me any kB, so I guess it was only used by bevy_rand and not rand


## Added more functionality, I think

made sure the thumbstick reading didn't start until after bevy was ready

added some state logging

huh, heap grew for a while but eventually settled down. I wonder when I'm going to get issues

I hope I'm able to write the rest!