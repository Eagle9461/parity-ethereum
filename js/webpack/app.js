
// Copyright 2015-2017 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

const webpack = require('webpack');
const path = require('path');
// const ReactIntlAggregatePlugin = require('react-intl-aggregate-webpack-plugin');
const WebpackErrorNotificationPlugin = require('webpack-error-notification');
const CopyWebpackPlugin = require('copy-webpack-plugin');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const ExtractTextPlugin = require('extract-text-webpack-plugin');
const ServiceWorkerWebpackPlugin = require('serviceworker-webpack-plugin');
const ScriptExtHtmlWebpackPlugin = require('script-ext-html-webpack-plugin');

const rulesEs6 = require('./rules/es6');
const rulesParity = require('./rules/parity');
const Shared = require('./shared');

const DAPPS_BUILTIN = require('@parity/shared/config/dappsBuiltin.json');
const DAPPS_VIEWS = require('@parity/shared/config/dappsViews.json').map((dapp) => {
  dapp.commons = true;
  return dapp;
});

const FAVICON = path.resolve(__dirname, '../node_modules/@parity/shared/assets/images/parity-logo-black-no-text.png');

const DEST = process.env.BUILD_DEST || '.build';
const ENV = process.env.NODE_ENV || 'development';
const EMBED = process.env.EMBED;

const isProd = ENV === 'production';
const isEmbed = EMBED === '1' || EMBED === 'true';
const isAnalize = process.env.WPANALIZE === '1';

const entry = isEmbed
  ? {
    embed: './embed.js'
  }
  : Object.assign({}, Shared.dappsEntry, {
    index: './index.js'
  });

module.exports = {
  cache: !isProd,
  devtool: isProd ? '#hidden-source-map' : '#source-map',

  context: path.join(__dirname, '../src'),
  entry: entry,
  output: {
    // publicPath: '/',
    path: path.join(__dirname, '../', DEST),
    filename: '[name].[hash:10].js'
  },

  module: {
    rules: [
      rulesParity,
      rulesEs6,
      {
        test: /\.js$/,
        exclude: /(node_modules)/,
        use: [ 'babel-loader' ]
      },
      {
        test: /\.json$/,
        use: [ 'json-loader' ]
      },
      {
        test: /\.ejs$/,
        use: [ 'ejs-loader' ]
      },
      {
        test: /\.html$/,
        use: [
          {
            loader: 'file-loader',
            options: {
              name: '[name].[ext]'
            }
          },
          'extract-loader',
          {
            loader: 'html-loader',
            options: {
              root: path.resolve(__dirname, '../assets/images'),
              attrs: ['img:src', 'link:href']
            }
          }
        ]
      },
      {
        test: /\.md$/,
        use: [ 'html-loader', 'markdown-loader' ]
      },
      {
        test: /\.css$/,
        include: [ /packages/, /src/, /@parity/ ],
        use: [
          'style-loader',
          {
            loader: 'css-loader',
            options: {
              importLoaders: 1,
              localIdentName: '[name]_[local]_[hash:base64:5]'
            }
          },
          {
            loader: 'postcss-loader',
            options: {
              'postcss-import': {},
              'postcss-nested': {},
              'postcss-simple-vars': {}
            }
          }
        ]
      },
      {
        test: /\.css$/,
        exclude: [ /packages/, /src/, /@parity/ ],
        use: [ 'style-loader', 'css-loader' ]
      },
      {
        test: /\.(png|jpg)$/,
        use: [ {
          loader: 'file-loader',
          options: {
            name: 'assets/[name].[hash:10].[ext]'
          }
        } ]
      },
      {
        test: /\.(woff|woff2|ttf|eot|otf)(\?v=[0-9]\.[0-9]\.[0-9])?$/,
        use: [ {
          loader: 'file-loader',
          options: {
            name: 'fonts/[name][hash:10].[ext]'
          }
        } ]
      },
      {
        test: /parity-logo-white-no-text\.svg/,
        use: [ 'url-loader' ]
      },
      {
        test: /\.svg(\?v=[0-9]\.[0-9]\.[0-9])?$/,
        exclude: [ /parity-logo-white-no-text\.svg/ ],
        use: [ {
          loader: 'file-loader',
          options: {
            name: 'assets/[name].[hash:10].[ext]'
          }
        } ]
      }
    ],
    noParse: [
      /node_modules\/sinon/
    ]
  },

  resolve: {
    alias: {
      '~': path.resolve(__dirname, '..'),
      '@parity/abi': path.resolve(__dirname, '../node_modules/@parity/abi'),
      '@parity/api': path.resolve(__dirname, '../node_modules/@parity/api'),
      '@parity/etherscan': path.resolve(__dirname, '../node_modules/@parity/etherscan'),
      '@parity/jsonrpc': path.resolve(__dirname, '../node_modules/@parity/jsonrpc'),
      '@parity/parity.js': path.resolve(__dirname, '../node_modules/@parity/parity.js'),
      '@parity/shared': path.resolve(__dirname, '../node_modules/@parity/shared'),
      '@parity/ui': path.resolve(__dirname, '../node_modules/@parity/ui'),
      '@parity/wordlist': path.resolve(__dirname, '../node_modules/@parity/wordlist'),
      '@parity': path.resolve(__dirname, '../packages')
    },
    modules: [
      path.join(__dirname, '../node_modules')
    ],
    extensions: ['.json', '.js', '.jsx'],
    unsafeCache: true
  },

  node: {
    fs: 'empty'
  },

  plugins: (function () {
    const DappsHTMLInjection = []
      .concat(DAPPS_BUILTIN, DAPPS_VIEWS)
      .filter((dapp) => !dapp.skipBuild)
      .map((dapp) => {
        return new HtmlWebpackPlugin({
          title: dapp.name,
          filename: dapp.url + '.html',
          template: '../packages/dapps/index.ejs',
          favicon: FAVICON,
          secure: dapp.secure,
          chunks: [ !isProd || dapp.commons ? 'commons' : null, dapp.url ]
        });
      });

    let plugins = Shared.getPlugins().concat(
      new WebpackErrorNotificationPlugin()
    );

    if (!isEmbed) {
      plugins = [].concat(
        plugins,

        new HtmlWebpackPlugin({
          title: 'Parity',
          filename: 'index.html',
          template: './index.ejs',
          favicon: FAVICON,
          chunks: [
            isProd ? null : 'commons',
            'index'
          ]
        }),

        new ServiceWorkerWebpackPlugin({
          entry: path.join(__dirname, '../src/serviceWorker.js')
        }),

        DappsHTMLInjection,

        new webpack.DllReferencePlugin({
          context: '.',
          manifest: require(`../${DEST}/vendor-manifest.json`)
        }),

        new ScriptExtHtmlWebpackPlugin({
          sync: [ 'commons', 'vendor.js' ],
          defaultAttribute: 'defer'
        }),

        new CopyWebpackPlugin([
          { from: './error_pages.css', to: 'styles.css' },
          { from: '../packages/dapps/static' }
        ], {})
      );
    }

    if (isEmbed) {
      plugins.push(
        new HtmlWebpackPlugin({
          title: 'Parity Bar',
          filename: 'embed.html',
          template: './index.ejs',
          favicon: FAVICON,
          chunks: [
            isProd ? null : 'commons',
            'embed'
          ]
        })
      );
    }

    if (!isAnalize && !isProd) {
      // const DEST_I18N = path.join(__dirname, '..', DEST, 'i18n');

      plugins.push(
        // new ReactIntlAggregatePlugin({
        //   messagesPattern: DEST_I18N + '/i18n/**/*.json',
        //   aggregateOutputDir: DEST_I18N + '/i18n/',
        //   aggregateFilename: 'en'
        // }),

        new webpack.optimize.CommonsChunkPlugin({
          filename: 'commons.[hash:10].js',
          name: 'commons',
          minChunks: 2
        })
      );
    }

    if (isProd) {
      plugins.push(new ExtractTextPlugin({
        filename: 'styles/[name].[hash:10].css',
        allChunks: true
      }));
    }

    return plugins;
  }())
};
