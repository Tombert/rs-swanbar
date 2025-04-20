# rs-swanbar

A custom Swaybar written in Rust designed to be modular and as low of memory as possible. 

## Why? 

Sway is a great window manager, but it feels a little incomplete out of the box.  The bar on the top, by default, doesn't have anything; no system clock, no battery indicator, no volume levels, no inspirational quotes from ChatGPT.  Useless. 

It's not too hard to add most of that with a basic bash script, but that gets increasingly clunky, so I felt it would be cool to write my own implementation that is somewhat pluggable and had support for lots of fun features like TTLs, non-blocking asynchronous tasks, and fully concurrent with persistence. 

This framework for this is fairly elaborate, with support for custom TTLs, timeouts, and as many modules as you would like. It can also serve as a simpling scheduling framework that doesn't affect the swaybar (look at the `bgchange` module for examples) to work as a something roughly like `cron` .


## Didn't you already build this?

This is a port of my [previous Swaybar implementation](https://github.com/Tombert/swanbar) that was written in Clojure. 
Using GraalVM I managed to get the memory of that one down to roughly 18 megabytes.  I was relatively happy with this, but I was curious how low I could get the memory usage, and after tweaking the GraalVM settings and doing what I could to reduce the memory Clojure was using, I was not able to significantly lower the memory from 18 megs, I think primarily because of the garbage collector.

I figured that this would be as good a time as any to finally learn Rust, since it doesn't require a garbage collector and has a reputation for being very fast. 

After several rewrites, the memory usage hovers around 500 kilobytes using the `nix build` path.  It compiles against `musl` and should be relatively portable.  

I have not tested this, but I believe this should work in `i3` with little to no modifications, but YMMV.
