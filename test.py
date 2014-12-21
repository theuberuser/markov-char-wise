import os
import ctypes


def get_lib_filename(base_dir, name):
    for fn in os.listdir(base_dir):
        if fn.startswith('lib{}-'.format(name)) and fn.endswith('.so'):
            return os.path.join(base_dir, fn)
    else:
        raise KeyError('no library found')

libmarkov = ctypes.CDLL(get_lib_filename('./target', 'markov'))

libmarkov.markov_alloc.restype = ctypes.c_void_p

libmarkov.markov_dealloc.argtypes = [ctypes.c_void_p]
libmarkov.markov_dealloc.restype = ctypes.c_int

libmarkov.markov_learn.argtypes = [
    ctypes.c_void_p, ctypes.c_char_p, ctypes.c_uint]
libmarkov.markov_learn.restype = ctypes.c_int

libmarkov.markov_speak.argtypes = [
    ctypes.c_void_p, ctypes.c_char_p, ctypes.c_uint]
libmarkov.markov_speak.restype = ctypes.c_int


class MarkovGenerator(object):
    def __init__(self):
        self._ptr = libmarkov.markov_alloc()
        if not self._ptr:
            raise Exception("allocation failure")

    def __del__(self):
        if self._ptr is not None:
            rv = libmarkov.markov_dealloc(self._ptr)
            if rv != 0:
                raise Exception("errno = {}".format(-rv))
        self._ptr = None

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
        return buffer_.raw[0:rv]

