package main

import (
	"context"
	"fmt"
	"github.com/go-redis/redis/v8"
	"github.com/savsgio/atreugo/v11"
	"strconv"
	"sync/atomic"
	"time"
)

var Count uint64
var KEY = "CLICKY_COUNTER"

func main() {
	ctx := context.Background()
	rdb := redis.NewClient(&redis.Options{
		Addr: ":6379",
	})
	config := atreugo.Config{
		Addr: "127.0.0.1:8080",
	}
	server := atreugo.New(config)

	go func() {
		previousRemoteCount := uint64(0)
		for {
			txf := func(tx *redis.Tx) error {
				// get current value or zero
				remoteCount, err := tx.Get(ctx, KEY).Uint64()
				if err != nil {
					return err
				}

				// runs only if the watched keys remain unchanged
				_, err = tx.Pipelined(ctx, func(pipe redis.Pipeliner) error {
					if remoteCount < previousRemoteCount {
						remoteCount = previousRemoteCount
					}

					remoteContribution := remoteCount - previousRemoteCount
					localCount := atomic.AddUint64(&Count, remoteContribution) - remoteContribution // mimic the Rust atomic add behavior
					localContribution := localCount - previousRemoteCount
					remoteCount += localContribution
					previousRemoteCount = remoteCount
					pipe.Set(ctx, KEY, strconv.FormatUint(previousRemoteCount, 10), 0)
					//println("incrementing the counter ", remoteCount, "???")
					return nil
				})
				return err
			}
			err := rdb.Watch(ctx, txf, KEY)
			if err != nil {
				fmt.Printf("Failed to increment the counter: %v\n", err)
			}
			time.Sleep(time.Second)
		}
	}()

	server.GET("/", func(ctx *atreugo.RequestCtx) error {
		return ctx.TextResponse(strconv.FormatUint(atomic.LoadUint64(&Count), 10))
	})

	server.POST("/", func(ctx *atreugo.RequestCtx) error {
		given, err := strconv.ParseUint(string(ctx.PostBody()), 10, 64)
		if err != nil || given > 500 {
			return ctx.TextResponse("Invalid value")
		}

		newValue := atomic.AddUint64(&Count, given)
		return ctx.TextResponse(strconv.FormatUint(newValue, 10))
	})

	if err := server.ListenAndServe(); err != nil {
		panic(err)
	}
}
