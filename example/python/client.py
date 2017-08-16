#!/usr/bin/env python
# -*- coding: utf-8 -*-
# example/python/test.py
# Copyright (C) 2017 authors and contributors (see AUTHORS file)
#
# This module is released under the MIT License.

"""WHAT"""

# ============================================================================
# Imports
# ============================================================================


# Stdlib imports
from enum import Enum
import socket

# Third-party imports
import msgpack

# Local imports

# ============================================================================
# Globals
# ============================================================================


BINDADDR = '127.0.0.1'
BINDPORT = 9999
BUFFER_SIZE = 1024

# ============================================================================
# Message enums
# ============================================================================


class MessageType(Enum):
    """MessageType"""
    #  A message initiating a request.
    Request = 0

    #  A message sent in response to a request.
    Response = 1

    #  A message notifying of some additional information.
    Notification = 2


class SessionType(Enum):
    """SessionType"""

    # Session used when an agent is starting.
    #
    # Indicates that authentication services are likely to not be available.
    # Only the GetKeyFile request is available within a bootstrap session.
    Boot = 0

    # Authentication is used to allow access to keyfiles.
    #
    # All request types are available within an authenticated session.
    Auth = 1


class AuthMessage(Enum):
    """AuthMessage"""

    # Retrieve the keyfile.
    #
    # Requires 1 argument: key. Only succeeds if the keyfile exists.
    GetKeyFile = 0

    # Create the keyfile.
    #
    # Requires 2 arguments: key, keyfile. Only succeeds if the keyfile does
    # not exist.
    CreateKeyFile = 1

    # Change only the keyfile
    #
    # Requires 2 arguments: key, new keyfile. Only succeeds if the keyfile
    # already exists.
    ChangeKeyFile = 2

    # Change only the key
    #
    # Requires 2 arguments: old key, new key. Only succeeds if the keyfile
    # already exists.
    ChangeKey = 3

    # Replace the keyfile
    #
    # Requires 3 arguments: Old key, new key, new keyfile. Only succeeds if
    # the keyfile already exists.
    ReplaceKeyFile = 4

    # Delete the keyfile.
    #
    # requires 1 argument: key. Only succeeds if the keyfile already exists.
    DeleteKeyFile = 5

    # Check if a key exists
    #
    # requires 1 argument: key. Always succeeds and returnes true or false.
    KeyExists = 6


class AuthError(Enum):
    """AuthError"""
    Nil = 0

    # Key file is not found.
    KeyFileNotFound = 1

    # Key file exists.
    KeyFileExists = 2

    # DB error
    DatabaseError = 3


# Used with the notification rpc message type.
class AuthNotice(Enum):
    """AuthNotice"""
    # No more requests will be made
    Done = 2


# ============================================================================
# Messages
# ============================================================================


def start_session():
    """start session"""
    msgtype = MessageType.Notification
    code = SessionType.Auth
    args = []
    msg = [msgtype.value, code.value, args]
    return msgpack.packb(msg)


def request(msgid, *args, method=None):
    """request"""
    if not isinstance(msgid, int):
        msg = 'msgid arg expected int, got {}'
        raise TypeError(msg.format(msgid.__class__.__name__))
    if not isinstance(method, AuthMessage):
        msg = 'method arg expected AuthMessage, got {}'
        raise TypeError(msg.format(type(method).__name__))

    msgtype = MessageType.Request
    args = list(args)

    # Create message
    msg = [msgtype.value, msgid, method.value, args]
    return msgpack.packb(msg)


def notify(*args, code=None):
    """notify"""
    if not isinstance(code, AuthNotice):
        msg = 'method arg expected AuthNotice, got {}'
        raise TypeError(msg.format(type(code).__name__))
    msgtype = MessageType.Notification
    args = list(args)

    # Create message
    msg = [msgtype.value, code.value, args]
    return msgpack.packb(msg)


def request_getkeyfile(msgid, key):
    """GetKeyFile"""
    if not isinstance(key, str):
        errmsg = ('key arg expected str, got {} '
                  'instead').format(type(key).__name__)
        raise TypeError(errmsg)

    key = key.encode('utf-8')
    return request(msgid, key, method=AuthMessage.GetKeyFile)


def request_createkeyfile(msgid, key, keyfile):
    """request_createkeyfile"""
    values = [('key', key), ('keyfile', keyfile)]
    for name, val in values:
        if not isinstance(val, str):
            errmsg = ('{} arg expected str, got {} '
                      'instead').format(name, type(val).__name__)
            raise TypeError(errmsg)

    key = key.encode('utf-8')
    keyfile = keyfile.encode('utf-8')

    return request(msgid, key, keyfile, method=AuthMessage.CreateKeyFile)


def notify_done():
    """notify_done"""
    return notify(code=AuthNotice.Done)


# ============================================================================
# Main
# ============================================================================


def main():
    """main"""
    # Setup connection
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.connect((BINDADDR, BINDPORT))

    # Start session
    s.send(start_session())

    # Send GetKeyFile
    s.send(request_getkeyfile(1, '42'))

    # # Send CreateKeyFile
    # s.send(request_createkeyfile)

    # Get response
    data = s.recv(BUFFER_SIZE)

    # Unpack
    response = msgpack.unpackb(data)

    print('Number of params in response', len(response))

    response[0] = MessageType(response[0]).name
    # response[1] is msgid
    response[2] = AuthError(response[2]).name
    response[3] = response[3].decode('utf-8')

    # Finish session
    s.send(notify_done())

    s.close()

    print(response)


if __name__ == '__main__':
    main()


# ============================================================================
#
# ============================================================================
