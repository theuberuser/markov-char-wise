import os
import ctypes
import codecs


def get_lib_filename(base_dir, name):
    for fn in os.listdir(base_dir):
        if fn == "lib{}.so".format(name):
            return os.path.join(base_dir, fn)
        if fn.startswith('lib{}-'.format(name)) and fn.endswith('.so'):
            return os.path.join(base_dir, fn)
    else:
        raise KeyError('no library found')

libmarkov = ctypes.CDLL(get_lib_filename('./target', 'markov'))

libmarkov.markov_alloc.restype = ctypes.c_void_p

libmarkov.markov_dealloc.argtypes = [ctypes.c_void_p]
libmarkov.markov_dealloc.restype = ctypes.c_int

libmarkov.markov_reply.argtypes = [
    ctypes.c_void_p,
    ctypes.c_char_p, ctypes.c_uint,
    ctypes.c_char_p, ctypes.c_uint
]
libmarkov.markov_reply.restype = ctypes.c_int

libmarkov.markov_learn.argtypes = [
    ctypes.c_void_p, ctypes.c_char_p, ctypes.c_uint]
libmarkov.markov_learn.restype = ctypes.c_int

libmarkov.markov_speak.argtypes = [
    ctypes.c_void_p, ctypes.c_char_p, ctypes.c_uint]
libmarkov.markov_speak.restype = ctypes.c_int


class MarkovGenerator(object):
    def __init__(self):
        self._lib = libmarkov
        self._ptr = self._lib.markov_alloc()
        if not self._ptr:
            raise Exception("allocation failure")

    def __del__(self):
        if self._ptr is not None:
            rv = self._lib.markov_dealloc(self._ptr)
            if rv != 0:
                raise Exception("errno = {}".format(-rv))
            self._ptr = None

    def reply(self, message):
        buf_type = ctypes.c_char * len(message)
        ibuf = buf_type(*list(message))
        obuf = ctypes.create_string_buffer(8192)
        rv = libmarkov.markov_reply(
            self._ptr,
            ibuf, ctypes.sizeof(ibuf),
            obuf, ctypes.sizeof(obuf))
        if rv < 0:
            raise Exception("errno = {}".format(-rv))
        return codecs.decode(obuf.raw[0:rv], 'utf8')

    def learn(self, message):
        buf_type = ctypes.c_char * len(message)
        buffer_ = buf_type(*list(message))
        rv = libmarkov.markov_learn(self._ptr, buffer_, ctypes.sizeof(buffer_))
        if rv < 0:
            raise Exception("errno = {}".format(-rv))

    def speak(self):
        buffer_ = ctypes.create_string_buffer(8192)
        rv = libmarkov.markov_speak(self._ptr, buffer_, ctypes.sizeof(buffer_))
        if rv < 0:
            raise Exception("errno = {}".format(-rv))
        return codecs.decode(buffer_.raw[0:rv], 'utf8')


test_data = """Too many cooks spoil the soup.  Or in this case, two of the cooks went off to work at a fast food place, leaving one cook behind to try and finish the soup
I know you're being sarcastic, but there are people who reply like that seriously. They actually think they only reason western animation died and chinks thrived is because of wages. There were great western guys before. But, now you've got kids in overpriced west coast art schools who think they're hot shit, when they'd really just get steamrolled by the simplest in-between job for studio DEEN.
So what's so terrible about streaming anime, /a/?
Currently living on campus at my uni and theyve been known crack down to crack down hard on any piracy they see. I could get my own router and ISP if I really wanted but I think I can deal with it for a semester
We (and basically everyone else) only care about unauthorized filesharing if we get a DMCA notice because of it (since we're basically an ISP they get sent to us and we're supposed to forward them once we've identified the person who did it). We don't sit and stare at the squid logs or something looking for bittorrent packet headers or something equally autistic.  Also relevant: a while ago, I did some analysis of >10k DMCA notices received over the past several years. As far as I can tell, there have been zero notices related to anime in any way. You're perfectly safe.
OP here. I have tried torrents but I don't understand how to use them, yeah. But I also see nothing wrong with streaming HD on a good laptop with good WiFi. No buffering and regardless of whether or not downloads look slightly better, it's still good quality.
Bitch was scary. What kind of child has absolutely no desires at all? I bet all Ruuko did all day was stare at the wall. When she was told to do homework, she did. When she was told to play outside, she did. She didn't have any foods she liked or disliked, she just ate everything and said it was good when asked. That's not normal.
Weak season, good ending. Nice to see everyone as humans, was disappointed they didn't go into Ruuko and her lack of any desire more, but that's more of an issue with the series overall after they gave her the I want to free everyone wish.
Second part of my post, her desire stemmed out of wanting to save everyone, or I guess that kick-started it. I was hoping for something different than the usual save em all mentality from the protagonist. That just felt off for me, but I guess I can accept that she wanted to save everyone because she couldn't stand, as someone with no desires, seeing others with wishes being destroyed.
I don't have any problems to say I kind of enjoy Okada's way to handle their characters. When they are not acting like fucking retards. Right now, I'm watching True Tears and I'm watching characters acting like humans.
I feel like Akira's conclusion was kind of shitty. Like, she's suddenly just sane and working in the modeling industry again? You don't usually recover from this kind of insanity.
"""

def get_populated_mg():
    mg = MarkovGenerator()
    for line in test_data.split("\n"):
        mg.learn(line)
    return mg

if __name__ == '__main__':
    mg = get_populated_mg()
    for _ in xrange(10):
        print("{}".format(mg.speak()))

