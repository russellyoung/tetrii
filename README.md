# tetrii
Project to learn about Rust

Every new language I try to learn I start with tetrii, multiple board tetris. I've written it in C with X-windows, C++, Java, 
Javascript, Python, and ELisp, assuming I haven't forgotten any others.

Despite is surface similarity to a bunch of other languages, I found the changes imposed by the different memory paradigm to be challenging. It took me much longer to get started on this, and my firs design, after getting about 90& done, had to be discarded and redone.

The current code is complete and works. Still, after a well-deserver break I will come back to look again. There are a few things I'm not completely happy with. One is the use of static mut variables. Getting my data structures into callback functions was challenging, so I ended up using static muts to hold things like the windows. (they aren't really muts, but since they are initted after the program starts they need to be assigned to). I have some ideas for how I can eliminate these through redesigning the modules. Maybe.

The other thing is the privacy implementations. There was no plan to it, and while at first things kept within a vague mental map, at the end a bunch of random functions needed to be exposed, and the statics had to be moved into main so they'd be available everywhere needed. Both of these are sloppy, and I'd like to see if with some minimal amount of experience now I Can do better. 

Planning also was lacking in assigning int types. I think it is important to sit down and map out the major variables, and decide ahead of time rather than when typing them in what they should be. 

Besides these improvements, there are a few bells and whistles I've added to other versions that could be added here as well. Saving the configuration to the config file is one, and allowing editing, saving, and selecting different keymaps should ot be too hard to do. We'll see, I definitely need a break from this for a while.
