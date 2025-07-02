import socket

tcp_socket = socket.create_connection(('127.0.0.1', 6378))

try:
    data = str.encode("*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n")
    tcp_socket.sendall(data)
    data = tcp_socket.recv(512)
    print(data)
    data = str.encode("*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n")
    tcp_socket.sendall(data)
    data = tcp_socket.recv(512)
    print(data)
    data = str.encode("*2\r\n$3\r\nGET\r\n$3\r\nnothere\r\n")
    tcp_socket.sendall(data)
    data = tcp_socket.recv(512)
    print(data)

finally:
    print("Closing socket")
    tcp_socket.close()
