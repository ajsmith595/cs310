*I actually started writing this tutorial back in September but have been hugely busy and not got round to posting it. Well, I managed to pull my finger out and get it online. Enjoy!*

Friends, a while back I posted a short tutorial to get you lovely people up and running using GStreamer and Python. I wrote the tutorial, partially so we get more people using GStreamer, and partially to encourage more people to hack on the awesome [Jokosher project](http://www.jokosher.org/). Right now I am sat on a plane bored out of my tiny little mind on the way to Florida for my hols, so I figured I would now write a tutorial about how to use GStreamer and Edward Hervey’s fantastic [Gnonlin](http://gnonlin.sourceforge.net/) set of elements.

As ever, feel free to use the comments on this blog entry to ask and answer questions. The last tutorial seemed to encourage some discussion, and I would love this one to do so too. So, lets crack on. 🙂

## What is Gnonlin exactly?

Ever since the birth of the GStreamer project, the developers have wanted it to be used for a variety of different purposes, including both media playback tools (such as Totem, Rhythmbox and Banshee) as well as media creation tools (such as Jokosher, PiTiVi and Buzztard). When building applications for creating media, many of them share the common theme of a timeline with individual clips on it that can be moved around, split, and played in different ways. These applications are called *non-linear* editors. In virtually all of these applications, the act of splitting the events on the timeline, moving them around and suchlike does NOT actually modify the original content. As an example, when you import a song into Jokosher and split it in half, Jokosher does not actually modify the original audio file – it instead rather cleverly figures out which bits to play so that the split exists when you hear it. This is called *non-destructive* editing, and this is what Gnonlin is here to help us with.

Gnonlin provides a collection of GStreamer elements that can be used in your GStreamer pipelines, and the elements help you to build non-linear editors that are non-destructive. Gnonlin is written by the awesome Edward Hervey, who is a core GStreamer developer. Gnonlin is used extensively in Jokosher and PiTiVi and there are sure to be more applications using it, particularly once you lovely people have read this tutorial and are feeling the Gnonlin love. 😛

## How Gnonlin works

Gnonlin has been well designed and is a fairly logical system to work with. The way gnonlin works can be fairly accurately tied to the concept of how a timeline works.

Imagine for a second you have a timeline with three clips on it (so it looks like a ruler with three blocks on it). In the timeline I import a song and chop it into chunks, so each chunk plays a separate little bit of the original song that I imported. Each chunk of audio plays one by one as the playhead moves across the timeline.

In Gnonlin, the container for a bunch of chunks of media (the timeline) is called a `gnlcomposition`. The `gnlcomposition` is a single container that in turn holds a bunch of chunks of media (such as our audio clips above). Each audio clip (or video clip if you like) is contained in a `gnlfilesource`. Technically, it is a `gnlsource` but the `gnlfilesource` element is used in virtually all cases. So, in our example above, each chunk of audio is a separate `gnlfilesource` and they all live in the same `gnlcomposition`. When you create your `gnlcomposition` and add your `gnlfilesource`s, you set some properties on each `gnlfilesource` to indicate which media file and which bits of that file should be played.

The humble `gnlfilesource` is quite a clever little thing. When you create one, you tell it which media file it should use, and it will construct a pipeline internally for you. As such, when you create a `gnlfilesource` you just tell it which file it should use (irrespective of format) and *thats it* – Gnonlin takes care of the heavy lifting of figuring out the relevant pipeline. This makes using Gnonlin a nice clean experience…and we all love nice clean experiences. No sniggering at the back please…

## Writing some code

Right, lets dig in and write some code. Once again, I am going to make the assumption that you are familiar with using a Glade GUI (if you are not see [this fantastic tutorial](http://www.learningpython.com/2006/05/30/building-an-application-with-pygtk-and-glade/). You should also be familiar with the basics of GStreamer and Python – see [my own tutorial](https://www.jonobacon.com/?p=750) for a primer on this.

Now, download the code for our first example:

* [Download gnonlin-tutorial1.py](https://www.jonobacon.com/files/gnonlin-tutorial1.py)
* [Download gui.glade](https://www.jonobacon.com/files/gui.glade)

This example will construct a `gnlcomposition`, put a `gnlfilesource` in it, and play a portion of the file. This is the kind of code that would be run when you have imported an audio file into the timeline in Jokosher and trimmed it so that only a portion of the file is left.

We are going to create the following approximate pipeline:

`gnlcomposition ( gnlfilesource ) ! audioconvert ! alsasink`

We will create a `gnlcomposition` that contains one or more `gnlfilesource` elements, and the `gnlcomposition` will hook up to an `audioconvert` before then hooking up to an `alsasink`.

Lets get started. First of all, import a bunch of things:

#!/usr/bin/python
import pygst
pygst.require(“0.10”)
import gst
import pygtk
import gtk
import gtk.glade

Note how we don’t import a different gnonlin module, the gnonlin elements are just normal GStreamer elements and part of GStreamer itself (although you do need to make sure you have installed the gnonlin package for your distribution). Now create a class and create its constructor:

class Main:
def __init__(self):

Then get the glade goodness going:

# set up the glade file
self.wTree = gtk.glade.XML(“gui.glade”, “mainwindow”)

signals = {
“on_play_clicked” : self.OnPlay,
“on_stop_clicked” : self.OnStop,
“on_quit_clicked” : self.OnQuit,
}

self.wTree.signal_autoconnect(signals)

Here we have three methods for the GUI to Play, Stop and Quit. We will look at these later. Now create a pipeline:

# creating the pipeline
self.pipeline = gst.Pipeline(“mypipeline”)

Then create a `gnlcomposition`:

# creating a gnlcomposition
self.comp = gst.element_factory_make(“gnlcomposition”, “mycomposition”)
self.pipeline.add(self.comp)
self.comp.connect(“pad-added”, self.OnPad)

Here you create the element and add it to the pipeline. You then create a callback for the `pad-added` signal that is part of the `gnlcomposition`. The reason for this is that the `gnlcomposition` has *dynamic pads*. Hold fire though, I will fill you in on the details about this later.

Now create the `audioconvert` and `alsasink` and add them to the pipeline:

# create an audioconvert
self.compconvert = gst.element_factory_make(“audioconvert”, “compconvert”)
self.pipeline.add(self.compconvert)

# create an alsasink
self.sink = gst.element_factory_make(“alsasink”, “alsasink”)
self.pipeline.add(self.sink)
self.compconvert.link(self.sink)

Notice how we link the `audioconvert` to the `alsasink`, but remember that we have not linked the `gnlcomposition` to the `audioconvert`. More on that later.

Now create a `gnlfilesource` element, and instead of adding it to the pipeline, remember that it needs to be part of the `gnlcomposition`, so we add it there:

# create a gnlfilesource
self.audio1 = gst.element_factory_make(“gnlfilesource”, “audio1”)
self.comp.add(self.audio1)

Right, this is where we delve into the specifics of which bits of the audio file are played. The `gnlfilesource` has a number of properties that can be set to indicate which bit of the audio file should be played at which time:

# set the gnlfilesource properties
self.audio1.set_property(“location”, “/home/jono/Desktop/jonobacon-littlecoalnose.ogg”)
self.audio1.set_property(“start”, 0 * gst.SECOND)
self.audio1.set_property(“duration”, 5 * gst.SECOND)
self.audio1.set_property(“media-start”, 10 * gst.SECOND)
self.audio1.set_property(“media-duration”, 5 * gst.SECOND)

Lets look at what these properties do. The first sets the media file:

* `location` – this is the location of the media file to be played. Each `gnlfilesource` uses one media file.

The remaining properties can be thought of in pairs. The first two (`start` and `duration`) refer to at what point on the timeline media should be played, and the second pair (`media-start` and `media-duration`) specify which bits of the actual media file in the `gnlfilesource` should be played. So lets look at them:

* `start` – the start point in which the media is played on the timeline
* `duration` – how long the media should be played for
* `media-start` – the start point in the media file to play the content
* `media-duration` – how long the content should be played for

If you look at the code above you can see that we specify a value (such as 10) and multiply it by `gst.SECOND` – this is a convenient way of referring to seconds in GStreamer. As such `10 * gst.SECOND` equates to 10 seconds.

Look at the properties we have set in the code and you can see that we specify the `start` time to be `0` and and the `duration` to be `5`. These properties say that in the timeline we will play something from 0 to 5 seconds. To specify what (because remember it does not have to be the first five seconds of the media file) we use the `media-start` and `media-duration` properties. Here we specify `10` for `media-start` and `5` for `media-duration`. As such, between 0 and 5 seconds in the timeline, we will play from 0.10 – 0.15 in the media file.

This is quite complex to get you head around at first, so re-read the above few paragraphs until you get the hang of it.

Right, now show the window:

# show the window
self.window = self.wTree.get_widget(“mainwindow”)
self.window.show_all()

We now need to create some callbacks. Lets first start with `OnPad()`. Earlier I deferred discussion of this, so lets look at it now. 🙂

In GStreamer there is a concept called *Dynamic Pads*. What this means is that with some objects in GStreamer, pads only get added when other things have been processed, and one such item is the `gnlcomposition`. Before the `gnlcomposition` can know what pads should be added, it needs to check which `gnlfilesource`s it has and automatically figure out which elements are required to process the media that you want to play. All of this happens automatically, but it means that you cannot assume that it will have a pad – once the `gnlcomposition` has figured out what it is doing with its `gnlfilesource`s it will then provide a pad. See [GStreamer Dynamic Pads, Explained](https://www.jonobacon.com/?p=810) for more details on this.

When you initialise the pipeline and set it to `READY`, `PAUSED` or `PLAYING`, the `gnlcomposition` does its processing and emits a ‘pad-added’ signal to announce the pad is ready. Earlier we set up a connection so that when this signal is emitted, we connect our `OnPad()` method to it. This is that method:

def OnPad(self, comp, pad):
print “pad added!”
convpad = self.compconvert.get_compatible_pad(pad, pad.get_caps())
pad.link(convpad)

The signal gives you a pad (referenced with `pad`) and with it we use `get_compatible_pad()` on our `audioconvert` element to return a pad that is compatible with our `gnlcomposition`. We then link the `gnlcomposition` pad to the `audioconvert` pad. Job done.

Now add the other methods:

def OnPlay(self, widget):
print “play”
self.pipeline.set_state(gst.STATE_PLAYING)

def OnStop(self, widget):
print “stop”
self.pipeline.set_state(gst.STATE_NULL)

def OnQuit(self, widget):
print “quitting”
gtk.main_quit()

Finally, set it off:

start=Main()
gtk.main()

When you run the script and press the Play button, you will hear hear five seconds of audio played (0 – 5 secs on the timeline) but the audio played is 0.10 – 0.15 secs in the media file – remember we set the `start`, `duration`, `media-start` and `media-duration` properties to make this happen.

Now, it gets particularly interesting when you add more than one `gnlfilesource` to your composition, and set different media times – you then feel how Gnonlin manages different clips of audio or video. Go and download [gnonlin-tutorial2.py](https://www.jonobacon.com/files/gnonlin-tutorial2.py) which adds the following additional `gnlfilesource`:

# create another gnlfilesource
self.audio2 = gst.element_factory_make(“gnlfilesource”, “audio2”)
self.comp.add(self.audio2)

# set the second gnlfilesource properties
self.audio2.set_property(“location”, “/home/jono/Desktop/jonobacon-littlecoalnose.ogg”)
self.audio2.set_property(“start”, 5 * gst.SECOND)
self.audio2.set_property(“duration”, 5 * gst.SECOND)
self.audio2.set_property(“media-start”, 0 * gst.SECOND)
self.audio2.set_property(“media-duration”, 5 * gst.SECOND)

This should all look familiar, but we have changed the property times to play the second `gnlfilesource1` from 5 – 10 seconds in the timeline (immediately after the first one), but to play 0.00 – 0.05 from the media file. Feel free to change the second `gnlfilesource` so it uses a different media file if you like. With two `gnlfilesource`s in the composition, you can imagine this like having a single timeline with two audio clips on it.

## Concluding

We have only just scraped the surface of Gnonlin, and this guide is intended to get you started and playing with it. Gnonlin supports a number of other cool things though:

* **Operations** – you can add a `gnloperation` which will apply whatever is in that operation to the audio. So, if you had a LADSPA audio effect in the `gnloperation` (in a similar way to how a `gnlfilesource` is in a `gnlcomposition`), it will apply that effect to the relevant portion. We use this in Jokosher to apply audio effects.
* **Priorities** – what happens if you have a few different `gnlfilesource`s in the same `gnlcomposition` and they are set to play at the same time? You can set a priority to determine which ones play.
* **Default Sources** – A *default source* is what is played in the gaps between your `gnlfilesource`s. In Gnonlin *something* must be playing at all times, and invariably you want something in-between your media clips. So, as an example, in Jokosher, between audio clips we want silence to be played. To do this, we created a default source that just plays silence. The default source is automatically played when nothing else is playing.

If I get some time I hope to write some additional tutorials to show some of these features. But, in the meantime, I hope this tutorial has been of some use to get you started, and remember you can always ask questions in `#gstreamer` on `irc.freenode.net` and you are welcome to post questions in the comments on this entry.

Good luck!