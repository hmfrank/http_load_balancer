# HTTP Load Balancer
Simple HTTP load balancer (sticky round-robin).

## How to Use
```shell script
# start some servers
$ cargo run --bin server 127.0.0.1:8081 &
$ cargo run --bin server 127.0.0.1:8082 &
$ cargo run --bin server 127.0.0.1:8083 &
$ cargo run --bin server 127.0.0.1:8084 &

# start the load balancer
$ cargo run --bin load_balancer 127.0.0.1:8080 \
    127.0.0.1:8081 \
    127.0.0.1:8082 \
    127.0.0.1:8083 \
    127.0.0.1:8084
```

To test the load balancer, send an HTTP request with your browser.

![answer from 127.0.0.1:8081](https://github.com/user-attachments/assets/aa3cc717-10a2-4c03-8d93-15a57d8f40e0)

Subsequent requests will be answered by different servers.

![answer from 127.0.0.1:8082](https://github.com/user-attachments/assets/cc4bdd79-91a9-48a8-8ed0-bdd8c1db6fdd)
![answer from 127.0.0.1:8083](https://github.com/user-attachments/assets/32f851ac-6076-4a73-933a-6c7c2e29dc2e)
![answer from 127.0.0.1:8084](https://github.com/user-attachments/assets/48d2bf2f-0fc5-4eab-a6fd-197d8d6bb6ff)

Go to the URL /session to create a session (receive a session cookie).
Once you've created a session,
    all subsequent requests will be routed to the same server.

![session created](https://github.com/user-attachments/assets/260d22dd-6b7b-4c79-910f-8f9561ba966a)
