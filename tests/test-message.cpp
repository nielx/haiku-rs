#include <Message.h>
#include <iostream>
using namespace std;

int
main(int argc, char** argv) {
	BMessage msg('abcd');
	msg.PrintToStream();
	uint8 *buffer = new uint8[msg.FlattenedSize()];
	msg.Flatten((char *)buffer, msg.FlattenedSize());
	cout << "let msg: Vec<u8> = vec!(";
	for (int i = 0; i < msg.FlattenedSize(); i++) {
		cout << unsigned(buffer[i]) << ", ";
	}
	cout << ");\n\n";
	return 0;
}