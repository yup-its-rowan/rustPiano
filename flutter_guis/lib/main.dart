import 'package:flutter/material.dart';

void main() {
  runApp(const MainApp());
}

class MainApp extends StatelessWidget {
  const MainApp({super.key});

  @override
  Widget build(BuildContext context) {
    //return const Image assetImage with no padding
    return Image.asset('images/snoopyChristmas.gif', width:660, fit: BoxFit.cover);

    //return const Image(image: AssetImage('images/snoopyChristmas.gif'));
  }
}
