import socket

tcp_socket = socket.create_connection(('127.0.0.1', 6378))

try:
    data = str.encode("*2\r\n$4\r\nECHO\r\n$6\r\nBANANA\r\n")
    tcp_socket.sendall(data)
    data = tcp_socket.recv(100)
    print(data)

finally:
    print("Closing socket")
    tcp_socket.close()
