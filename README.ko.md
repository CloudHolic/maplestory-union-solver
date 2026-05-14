# maplestory-union-solver

메이플스토리 유니온 배치판 솔버. 브라우저에서 동작한다.

[English README](README.md)

## 무엇인가

**메이플스토리 유니온**은 MMORPG 메이플스토리의 메타 성장 시스템이다.
플레이어가 가진 각 캐릭터는 폴리오미노(1~5칸, 레벨에 따라) 블록으로
표현되며, 이 블록들을 공유 판에 배치하여 스탯 보너스를 얻는다. 배치 문제에는
실제로 다음과 같은 제약이 있다:

- 최대 42개의 캐릭터 블록을 판에 배치해야 함
- 각 블록에는 지정된 "마킹 셀"이 있으며, 적어도 하나의 마킹 셀은 판 중앙
  4칸 안에 와야 함
- 판 위의 명명된 영역(그룹)마다 개별 목표 칸 수가 있음
- 배치된 모든 블록은 하나의 연결 성분을 이뤄야 함

손으로 유효한 배치(하물며 최적 배치)를 찾는 것은 번거롭다. 이 프로젝트는
브라우저에서 완전히 실행되며 모든 제약을 만족하는 배치를 반환하는 솔버를
제공한다.

## 기술 스택

| 구성 요소 | 기술 |
|---|---|
| 프론트엔드 | React 19 + Vite + TypeScript |
| 솔버 코어 | Rust, WebAssembly로 컴파일 |
| ML 추론 (WASM 내부) | [`tract`](https://github.com/sonos/tract) (순수 Rust ONNX 런타임) |
| ML 학습 (개발 시에만) | Python + PyTorch (POSET 모델) |
| 배포 | Docker + nginx, Cloudflare Tunnel을 통한 셀프 호스팅 (Slice 6 예정) |

모든 솔빙 계산은 클라이언트 측에서 일어난다. 백엔드는 캐릭터 정보 조회를 위한
NEXON Open API 프록시 역할만 하며 솔빙에는 관여하지 않는다.

## 레포 구조

```
frontend/     React 프론트엔드
backend/      Go 백엔드 (NEXON Open API 프록시 + 캐시)
solver/       Rust 솔버 (WASM + native 타겟) + ML 데이터 생성기
poset/        Python ML 학습 파이프라인 (개발 시에만)
etl/          JSONL → Parquet → HF Hub 변환 도구 (개발 시에만)
models/       학습된 ONNX 모델
docs/         아키텍처 및 알고리즘 문서
```

각 서브 디렉터리에는 빌드 방법을 담은 고유한 `README.md`가 있다.

## 문서

- [아키텍처](docs/architecture.md)
- [ExactCover 알고리즘](docs/algorithms/exact-cover.md)
- [ML 피처 설계](docs/poset/features.md)

## 라이선스

이 레포는 디렉터리별로 다른 라이선스를 사용한다.
매핑은 [`LICENSE`](LICENSE) 참조.