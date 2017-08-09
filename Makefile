default: cheevoshash

cheevoshash:
	gcc src/cheevoshash.c -o bin/cheevoshash
	
clean:
	-rm -f bin/cheevoshash
