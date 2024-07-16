import json
import socket
import os

class Resource:
    def __init__(self, name, data):
        self.__name__ = name
        self.__data__ = data

    def __getattr__(self, key):
        return getattr(self.__data__, key)

    def __setattr__(self, key, value):
        if key in ["__name__", "__data__"]:
            return object.__setattr__(self, key, value)
        else:
            return setattr(self.__data__, key, value)

    def __repr__(self):
        return f"{self.__name__} = {repr(self.__data__)}"

    def __str__(self):
        return str(self.__data__)

class Struct:
    def __init__(self, data):
        self.__dict__ = data

    def __setattr__(self, key, value):
        getattr(self, key)
        super().__setattr__(key, value)

    def __repr__(self):
        return json.dumps(self, default=lambda x: x.__dict__, indent=2)

    def __str__(self):
        return json.dumps(self, default=lambda x: x.__dict__)

class Tyr:
    def load(self, **kwargs):
        packet = b''

        while True:
            packet += self.sock.recv(8192)

            try:
                return json.loads(packet.decode(), **kwargs)
            except json.decoder.JSONDecodeError:
                pass

    def __enter__(self):
        run = os.environ.get("XDG_RUNTIME_DIR", "/tmp")
        path = os.environ.get("TYR_SOCK", f"{run}/tyr.sock")

        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.connect(path)

        return self

    def __exit__(self, *args):
        self.sock.close()
        self.sock = None

    def resources(self):
        self.sock.send(b'"resources"')
        return self.load()

    def systems(self):
        self.sock.send(b'"systems"')
        return self.load()

    def get(self, name):
        self.sock.send(json.dumps({"get":name}).encode())
        return Resource(name, self.load(object_hook=Struct))

    def set(self, resource):
        self.sock.send(json.dumps({
            "set": {
                "name": resource.__name__,
                "data": resource.__data__,
            }
            }, default=lambda x: x.__dict__).encode())

    def enable(self, name):
        self.sock.send(json.dumps({"enable":name}).encode())

    def disable(self, name):
        self.sock.send(json.dumps({"disable":name}).encode())

def resources():
    with Tyr() as tyr:
        return tyr.resources()

def systems():
    with Tyr() as tyr:
        return tyr.systems()

def get(name):
    with Tyr() as tyr:
        return tyr.get(name)

def set(resource):
    with Tyr() as tyr:
        return tyr.set(resource)

def enable(resource):
    with Tyr() as tyr:
        return tyr.enable(resource)

def disable(resource):
    with Tyr() as tyr:
        return tyr.disable(resource)

def modify(name):
    class Modify:
        def __init__(self, name):
            self.name = name

        def __enter__(self):
            self.tyr = Tyr().__enter__()
            self.res = self.tyr.get(self.name)

            return self.res

        def __exit__(self, *args):
            self.tyr.set(self.res)
            self.tyr.__exit__(self, *args)

    return Modify(name)
