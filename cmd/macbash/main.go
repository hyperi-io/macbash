// Project:   macbash
// File:      cmd/macbash/main.go
// Purpose:   CLI entry point
// Language:  Go
//
// License:   Apache-2.0
// Copyright: (c) 2025 HyperSec Pty Ltd

package main

import (
	"fmt"
	"os"

	"github.com/hypersec-io/macbash/internal/cli"
)

var (
	version   = "dev"
	commit    = "unknown"
	buildTime = "unknown"
)

func main() {
	if err := cli.Execute(version, commit, buildTime); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}
