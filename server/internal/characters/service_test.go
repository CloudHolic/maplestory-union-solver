package characters

import (
	"context"
	"testing"
	"time"

	"github.com/CloudHolic/maplestory-union-solver/server/internal/nexon"
)

type fakeNexon struct {
	ocidCalls  int
	unionCalls int
	ocidValue  string
	unionValue *nexon.UnionData
}

func (f *fakeNexon) GetOCID(_ context.Context, _ string) (string, error) {
	f.ocidCalls++
	return f.ocidValue, nil
}

func (f *fakeNexon) GetUnionData(_ context.Context, _ string) (*nexon.UnionData, error) {
	f.unionCalls++
	return f.unionValue, nil
}

func sampleUnion() *nexon.UnionData {
	return &nexon.UnionData{
		UsePresetNo: 1,
		Presets: [][]nexon.Block{
			{{Type: "전사", Class: "히어로", Level: 250}},
			{}, {}, {}, {},
		},
	}
}

func newTestService(t *testing.T, fake *fakeNexon, ttl time.Duration, now time.Time) *Service {
	t.Helper()
	t.Skip("placeholder — see notes below for hermetic test setup")
	return nil
}

func TestService_FirstFetch_HitsBothEndpoints(t *testing.T) {
	t.Skip("enable once test helper is in place")
	fake := &fakeNexon{ocidValue: "ocid-1", unionValue: sampleUnion()}
	now := time.Unix(1_700_000_000, 0)
	svc := newTestService(t, fake, 60*time.Second, now)

	view, err := svc.GetByNickname(context.Background(), "newbie")
	if err != nil {
		t.Fatalf("unexpected: %v", err)
	}
	if fake.ocidCalls != 1 || fake.unionCalls != 1 {
		t.Errorf("calls: ocid=%d union=%d, want 1/1", fake.ocidCalls, fake.unionCalls)
	}
	if view.OCID != "ocid-1" {
		t.Errorf("ocid: got %q want ocid-1", view.OCID)
	}
}
