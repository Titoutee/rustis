import socket
import time

tcp_socket = socket.create_connection(('127.0.0.1', 6378))

try:
    # Command: SET foo bar
    data = str.encode("*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n")
    tcp_socket.sendall(data)
    data = tcp_socket.recv(512)
    print(data)

    # Command: GET foo
    data = str.encode("*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n")
    tcp_socket.sendall(data)
    data = tcp_socket.recv(512)
    print(data)

    # Command: GET nothere
    data = str.encode("*2\r\n$3\r\nGET\r\n$3\r\nnothere\r\n")
    tcp_socket.sendall(data)
    data = tcp_socket.recv(512)
    print(data)

    print("----------------------")

    data = str.encode("*5\r\n$3\r\nSET\r\n$3\r\nlol\r\n$3\r\nbar\r\n$2\r\nPX\r\n$4\r\n5000\r\n")
    tcp_socket.sendall(data)
    data = tcp_socket.recv(512)
    print(data)

    data = str.encode("*2\r\n$3\r\nGET\r\n$3\r\nlol\r\n")
    tcp_socket.sendall(data)
    data = tcp_socket.recv(512)
    print(data)

    print("begin of sleep")
    time.sleep(6)
    print("end of sleep")

    data = str.encode("*2\r\n$3\r\nGET\r\n$3\r\nlol\r\n")
    tcp_socket.sendall(data)
    data = tcp_socket.recv(512)
    print(data)

    print("---------------------")



finally:
    print("Closing socket")
    tcp_socket.close()
