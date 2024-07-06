package main

/*
#include <stdint.h>
typedef struct{
	unsigned char* client_proof;
	size_t client_proof_len;
	unsigned char* client_ephemeral;
	size_t client_ephemeral_len;
	unsigned char* expected_server_proof;
	size_t expected_server_proof_len;
} SRPAuthResult;

void free(void*);

*/
import "C"

import (
	"fmt"
	"unsafe"

	"github.com/ProtonMail/go-srp"
)

//export SRPAuth
func SRPAuth(username string, password []byte, version int, salt string, modulus string, serverEphemeral string, result *C.SRPAuthResult) *C.char {
	srpAuth, err := srp.NewAuth(version, username, password, salt, modulus, serverEphemeral)
	if err != nil {
		return C.CString(fmt.Sprintf("%v", err))
	}

	proofs, err := srpAuth.GenerateProofs(2048)
	if err != nil {
		return C.CString(fmt.Sprintf("%v", err))
	}

	clientProof, s1 := sliceToCMem(proofs.ClientProof)
	clientEphemeral, s2 := sliceToCMem(proofs.ClientEphemeral)
	expectedServerProof, s3 := sliceToCMem(proofs.ExpectedServerProof)

	result.client_proof = clientProof
	result.client_proof_len = s1
	result.client_ephemeral = clientEphemeral
	result.client_ephemeral_len = s2
	result.expected_server_proof = expectedServerProof
	result.expected_server_proof_len = s3

	return nil
}

//export CGoFree
func CGoFree(ptr *C.void) {
    C.free(unsafe.Pointer(ptr))
}

func sliceToCMem(slice []byte) (*C.uchar, C.size_t) {
	cBuf := C.CBytes(slice)
	return (*C.uchar)(cBuf), C.size_t(len(slice))
}

func main() {}
