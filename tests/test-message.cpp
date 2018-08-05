#include <Message.h>
#include <iostream>
using namespace std;

void
print_to_stream(BMessage &msg) {
	msg.PrintToStream();
	uint8 *buffer = new uint8[msg.FlattenedSize()];
	msg.Flatten((char *)buffer, msg.FlattenedSize());
	cout << "let msg: Vec<u8> = vec!(";
	for (int i = 0; i < msg.FlattenedSize(); i++) {
		cout << unsigned(buffer[i]) << ", ";
	}
	cout << ");\n\n";
}


int
main(int argc, char** argv) {
	BMessage msg('abcd');
	print_to_stream(msg);
	BMessage msg2('efgh');
	msg2.AddUInt8("UInt8", 'a');
	print_to_stream(msg2);
	return 0;
}
