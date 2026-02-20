## 🏗️ Step 1: Blocking I/O & Echo Server

가장 기본적인 소켓 통신의 흐름을 이해하는 단계입니다.

* **Socket API 이해:** `bind`, `listen`, `accept`, `read`, `write`의 시스템 콜 흐름을 익혀야 합니다.
* **Rust의 `std::net`:** `TcpListener`와 `TcpStream` 사용법을 숙지하세요.
* **동기식 처리의 한계:** 한 클라이언트가 연결되어 `read`에서 대기(Blocking) 중일 때, 왜 다른 클라이언트가 접속하지 못하는지 그 **논리적 병목**을 파악하는 것이 중요합니다.
* *부족할 수 있는 부분:* 이때 "스레드를 무한정 늘리면 되지 않나?"라는 생각이 들 수 있습니다. 하지만 스레드 생성 비용과 **Context Switching** 오버헤드에 대해 미리 고민해 보세요.



## 🚀 Step 2: `epoll` & Non-blocking I/O

단일 스레드로 수만 개의 연결(C10K 문제)을 처리하는 핵심 마법입니다.

* **File Descriptor (FD):** 리눅스에서 모든 것은 파일이며, 소켓 또한 FD로 관리된다는 점을 이해해야 합니다.
* **Edge Triggered (ET) vs Level Triggered (LT):** `epoll`의 두 가지 동작 방식 차이를 명확히 알아야 데이터 유실을 막을 수 있습니다.
* **Event Loop:** 무한 루프 내에서 `epoll_wait`를 호출하고, 이벤트가 발생한 FD만 골라 처리하는 구조를 설계해야 합니다.
* **Rust 저수준 바인딩:** Rust 표준 라이브러리는 `epoll`을 직접 노출하지 않습니다. `libc` 크레이트를 사용해 직접 시스템 콜을 호출하거나, 입문용으로 `mio` 크레이트의 소스코드를 분석해 보는 것을 추천합니다.

## 🧵 Step 3: Thread Pool & Worker-Acceptor Pattern

I/O는 빠르지만, 비즈니스 로직(계산)이 무거워지면 전체 서버가 느려집니다.

* **관심사 분리:** 네트워크 이벤트를 받는 'Acceptor/Reactor' 스레드와 실제 연산을 수행하는 'Worker' 스레드 풀을 분리하는 아키텍처를 설계하세요.
* **MPSC (Multi-Producer, Single-Consumer) Queue:** 스레드 간에 작업(Task)을 안전하게 전달하기 위한 채널(Channel) 개념이 필요합니다.
* **Send & Sync Trait:** Rust에서 스레드 간 데이터를 안전하게 넘기기 위해 반드시 이해해야 하는 핵심 개념입니다. 이 벽을 넘지 못하면 컴파일 에러의 늪에 빠질 수 있습니다.

## 📨 Step 4: Protocol & Serialization

데이터를 어떻게 규격화해서 보낼 것인가의 문제입니다.

* **Framing:** TCP는 스트림 지향 프로토콜이라 데이터의 경계가 없습니다. "어디서부터 어디까지가 한 패킷인가?"를 정의하는 방법(Length-prefixing 등)을 배우세요.
* **Endianness:** 네트워크 바이트 순서(Big-Endian)와 호스트 바이트 순서에 대한 이해가 필요합니다.
* **Zero-copy Deserialization:** 성능 극대화를 위해 메모리 복사를 최소화하는 `rkyv`나 `serde`와 같은 라이브러리의 최적화 기법을 살펴보세요.